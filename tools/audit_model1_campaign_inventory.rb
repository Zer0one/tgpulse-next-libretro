#!/usr/bin/env ruby
# frozen_string_literal: true

# Check the complete screenshot/YAML/sample roster before workbook export.
require "pathname"
require "set"
require "yaml"

root = Pathname.new(__dir__).parent
base = root / "validation/nvram-campaigns/2026-10-01"
expected = {"vf" => [13, 98], "vr" => [10, 63], "vformula" => [9, 59],
            "swa" => [11, 92], "swaj" => [11, 90],
            "wingwar" => [14, 134], "wingwaru" => [14, 134],
            "wingwarj" => [14, 134], "wingwar360" => [14, 149]}
errors = []
total_fields = 0
total_values = 0

expected.each do |set, (field_count, value_count)|
  file = root / "data/diagnostic-menus/#{set}.yaml"
  unless file.file?
    errors << "#{set}: missing YAML"
    next
  end
  doc = YAML.safe_load(file.read, permitted_classes: [], aliases: true)
  errors << "#{set}: YAML set mismatch" unless doc.dig("game", "set") == set
  options = doc.fetch("menus").values.flat_map { |page| page.fetch("options") }
  count = options.sum { |item| item.fetch("observed_values").length }
  total_fields += options.length
  total_values += count
  errors << "#{set}: #{options.length} fields != #{field_count}" unless options.length == field_count
  errors << "#{set}: #{count} values != #{value_count}" unless count == value_count
  options.each do |item|
    key = item.fetch("key")
    errors << "#{set}/#{key}: incomplete sample status" unless item["sample_status"] == "all_values_saved_and_reloaded_direct_abi"
    item.fetch("observed_values").each_index do |step|
      screenshot = if key == "network"
                     %w[network-0 network-master network-slave].fetch(step)
                   else
                     "#{key.tr('_', '-')}-#{set == 'vr' ? format('%02d', step) : step}"
                   end
      errors << "#{set}/#{key}/#{step}: missing screenshot" unless
        (root / "docs/diagnostic-evidence/#{set}/#{screenshot}.png").file?
    end
  end
  samples = base / set / "samples"
  reloads = base / set / "reloads"
  sample_count = samples.directory? ? samples.children.count(&:directory?) : 0
  reload_count = reloads.directory? ? reloads.children.count(&:directory?) : 0
  errors << "#{set}: #{sample_count} sample directories != #{value_count}" unless sample_count == value_count
  errors << "#{set}: #{reload_count} reload directories != #{value_count}" unless reload_count == value_count
  next unless set.start_with?("wingwar")

  calibration = doc.dig("calibration", "volume_setting")
  errors << "#{set}: missing calibrated full-range sample" unless
    calibration && calibration["sample_status"] == "full_range_saved_and_reloaded_direct_abi" &&
    (base / set / "calibration/sample/committed.srm").file? &&
    (base / set / "calibration/reload/volume-default.srm").file?
end
errors << "total fields #{total_fields} != 110" unless total_fields == 110
errors << "total values #{total_values} != 953" unless total_values == 953
abort errors.join("\n") unless errors.empty?
puts "Model 1 campaign: 9 sets, #{total_fields} fields, #{total_values} saved values, 4 calibrated ranges"
