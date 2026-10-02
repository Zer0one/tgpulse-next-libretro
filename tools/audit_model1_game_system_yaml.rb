#!/usr/bin/env ruby
# frozen_string_literal: true

# Reconcile the VR / Virtua Formula YAML byte map with committed samples.
require "pathname"
require "set"
require "yaml"

set, sample_root = ARGV
abort "Usage: ruby #{File.basename(__FILE__)} vr|vformula /absolute/sample-dir" unless
  %w[vr vformula].include?(set) && sample_root
root = Pathname.new(__dir__).parent
data = YAML.safe_load((root / "data/diagnostic-menus/#{set}.yaml").read,
                      permitted_classes: [], aliases: true)
abort "YAML set mismatch" unless data.dig("game", "set") == set
options = data.dig("menus", "game_system", "options") or abort "No Game System options"
samples = Pathname.new(sample_root)
errors = []
expected_dirs = Set.new
common_baseline = nil
count = 0

options.each do |item|
  key = item.fetch("key")
  values = item.fetch("observed_values")
  mapping = item.fetch("nvram")
  offsets = mapping["eeprom_offsets"] || [mapping.fetch("eeprom_offset")]
  encoded = mapping.fetch("values")
  errors << "#{key}: byte map and visible values differ in length" unless encoded.length == values.length
  errors << "#{key}: sample status not complete" unless item["sample_status"] == "all_values_saved_and_reloaded_direct_abi"
  values.each_index do |step|
    name = "#{set}--#{key}--step-#{format('%02d', step)}"
    expected_dirs << name
    path = samples / name / "saved.srm"
    unless path.file?
      errors << "#{name}: missing saved sample"
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
    end
    count += 1
  end
end

actual_dirs = samples.children.select(&:directory?).map(&:basename).map(&:to_s).to_set
extra = actual_dirs - expected_dirs
errors << "unexpected sample directories: #{extra.to_a.sort.join(', ')}" unless extra.empty?
abort errors.join("\n") unless errors.empty?
puts "#{set}: #{options.length} YAML fields and #{count} committed samples agree byte-for-byte"
