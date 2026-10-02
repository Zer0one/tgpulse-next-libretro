#!/usr/bin/env ruby
# frozen_string_literal: true

# Reconcile SWA / Wing War YAML encodings with native-committed Save RAM.
require "pathname"
require "set"
require "yaml"

set, sample_root = ARGV
counts = {"swa" => 92, "swaj" => 90, "wingwar" => 134, "wingwaru" => 134,
          "wingwarj" => 134, "wingwar360" => 149}
abort "Usage: ruby #{File.basename(__FILE__)} SET /absolute/sample-dir" unless counts.key?(set) && sample_root
root = Pathname.new(__dir__).parent
doc = YAML.safe_load((root / "data/diagnostic-menus/#{set}.yaml").read,
                     permitted_classes: [], aliases: true)
abort "YAML set mismatch" unless doc.dig("game", "set") == set
options = doc.fetch("menus").values.flat_map { |page| page.fetch("options") }
samples = Pathname.new(sample_root)
counter = set.start_with?("swa") ? 11 : 10
errors = []
expected_dirs = Set.new
common_baseline = nil
count = 0

options.each do |item|
  key = item.fetch("key")
  mapping = item.fetch("nvram")
  offsets = (mapping["eeprom_offsets"] || [mapping.fetch("eeprom_offset")]).map do |offset|
    offset.is_a?(String) ? Integer(offset, 0) : offset
  end
  values = item.fetch("observed_values")
  encoded = mapping.fetch("values")
  errors << "#{key}: map and visible values differ in length" unless encoded.length == values.length
  errors << "#{key}: sample status not complete" unless item["sample_status"] == "all_values_saved_and_reloaded_direct_abi"
  values.each_index do |step|
    name = "#{set}--#{key}--step-#{format('%02d', step)}"
    expected_dirs << name
    path = samples / name / "saved.srm"
    unless path.file?
      errors << "#{name}: missing sample"
      next
    end
    raw = path.binread
    unless raw.bytesize == 65_728 && raw.start_with?("TGP1SRAM")
      errors << "#{name}: invalid Save RAM container"
      next
    end
    eeprom = raw.byteslice(65_600, 128).bytes
    actual = offsets.map { |offset| eeprom.fetch(offset) }
    expected = Array(encoded.fetch(step))
    errors << "#{name}: EEPROM #{actual.inspect} != YAML #{expected.inspect}" unless actual == expected
    if step.zero?
      common_baseline ||= eeprom
      errors << "#{name}: default EEPROM differs from set baseline" unless eeprom == common_baseline
    elsif common_baseline
      changed = (6...60).select { |i| eeprom[i] != common_baseline[i] }
      unexpected = changed - offsets - [8, 9, counter]
      errors << "#{name}: unrelated primary bytes changed #{unexpected.inspect}" unless unexpected.empty?
    end
    count += 1
  end
end

actual_dirs = samples.children.select(&:directory?).map(&:basename).map(&:to_s).to_set
extra = actual_dirs - expected_dirs
errors << "unexpected sample directories: #{extra.to_a.sort.join(', ')}" unless extra.empty?
errors << "sample count #{count} != #{counts.fetch(set)}" unless count == counts.fetch(set)
abort errors.join("\n") unless errors.empty?
puts "#{set}: #{options.length} YAML fields and #{count} committed samples agree byte-for-byte"
