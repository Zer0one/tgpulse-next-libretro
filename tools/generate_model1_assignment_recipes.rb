#!/usr/bin/env ruby
# frozen_string_literal: true

# Expand screenshot-verified SWA / Wing War assignments into isolated captures.
require "json"
require "pathname"
require "yaml"

root = Pathname.new(__dir__).parent
set, output, field_filter = ARGV
sets = %w[swa swaj wingwar wingwaru wingwarj wingwar360]
abort "Usage: ruby #{File.basename(__FILE__)} SET /absolute/output-dir [field-key]" unless
  sets.include?(set) && output && Pathname.new(output).absolute? && ARGV.size.between?(2, 3)
destination = Pathname.new(output).expand_path
abort "Output must be outside the repository" if destination == root || destination.to_s.start_with?(root.to_s + "/")
abort "Output already exists" if destination.exist?
doc = YAML.safe_load((root / "data/diagnostic-menus/#{set}.yaml").read,
                     permitted_classes: [], aliases: true)
abort "YAML set mismatch" unless doc.dig("game", "set") == set
wing = set.start_with?("wingwar")
menus = doc.fetch("menus")
expected_menus = wing ? %w[communication_setting game_assignments coin_assignments manual_setting] : %w[game_assignments coin_assignments manual_setting]
abort "Unexpected menu inventory" unless menus.keys == expected_menus
items = menus.flat_map do |name, page|
  options = page.fetch("options")
  abort "Incomplete #{name} positions" unless options.map { |item| item.fetch("position") } == (1..options.length).to_a
  options.map { |item| [name, item] }
end
abort "Unknown field key" if field_filter && items.none? { |_, item| item.fetch("key") == field_filter }
items.select! { |_, item| item.fetch("key") == field_filter } if field_filter
abort "Value cycle not closed" unless items.all? { |_, item| item["values_status"] == "cycle_closed" }
abort "Default differs from cycle start" unless items.all? do |_, item|
  item.fetch("native_default").to_s == item.fetch("observed_values").first.to_s
end
items.each do |_, item|
  item.fetch("observed_values").each_index do |step|
    basename = if item.fetch("key") == "network"
                 %w[network-0 network-master network-slave].fetch(step)
               else
                 "#{item.fetch('key').tr('_', '-')}-#{step}"
               end
    image = root / "docs/diagnostic-evidence/#{set}/#{basename}.png"
    abort "Missing documentary screenshot #{image}" unless image.file?
  end
end

def frame(actions, count, button = nil)
  action = {"frames" => count}
  action["buttons"] = [button] if button
  actions << action
end

def pulse(actions, button, release = 20)
  frame(actions, 10, button)
  frame(actions, release)
end

def enter_menu(actions, set, name)
  wing = set.start_with?("wingwar")
  frame(actions, 1200)
  frame(actions, 60, "L3")
  frame(actions, 60)
  root_steps = if wing
                 {"game_assignments" => 6, "coin_assignments" => 7, "manual_setting" => 7,
                  "communication_setting" => 12}.fetch(name)
               else
                 {"game_assignments" => 7, "coin_assignments" => 8, "manual_setting" => 8}.fetch(name)
               end
  root_steps.times { pulse(actions, "R3") }
  pulse(actions, "L3", 60)
  return unless name == "manual_setting"

  coin_fields = set == "wingwar360" ? 2 : 3
  (coin_fields + 1).times { pulse(actions, "R3") } # MANUAL SETTING
  pulse(actions, "L3", 60)
end

def navigate(actions, set, name, position)
  initial_first = name == "manual_setting" && !set.start_with?("wingwar")
  (initial_first ? position - 1 : position).times { pulse(actions, "R3") }
end

def actions_for(set, name, position, field_count, step)
  actions = []
  enter_menu(actions, set, name)
  navigate(actions, set, name, position)
  if step
    step.times { pulse(actions, "L3", 30) }
    actions << {"capture" => "selected"}
    # EXIT follows the last field. Coin also has a MANUAL SETTING action.
    exit_position = field_count + (name == "coin_assignments" ? 2 : 1)
    (exit_position - position).times { pulse(actions, "R3") }
    pulse(actions, "L3", 60)
    if name == "manual_setting"
      pulse(actions, "R3") unless set.start_with?("wingwar") # SWA returns with MANUAL SETTING selected
      pulse(actions, "L3", 60) # Coin EXIT
    end
    pulse(actions, "L3", 60) if set.start_with?("wingwar") # root EXIT
    actions << {"capture" => "saved"}
  else
    actions << {"capture" => "reloaded"}
  end
  actions
end

def toml(set, name, item, step, actions)
  lines = ["set = #{set.to_json}", "menu = #{name.to_json}", "field = #{item.fetch('key').to_json}"]
  lines += ["expected_value = #{item.fetch('observed_values')[step].to_s.to_json}", "value_step = #{step}"] if step
  actions.each do |action|
    lines << "" << "[[actions]]"
    if action.key?("capture")
      lines << "capture = #{action.fetch('capture').to_json}"
    else
      lines << "frames = #{action.fetch('frames')}"
      lines << "buttons = #{action.fetch('buttons').to_json}" if action.key?("buttons")
    end
  end
  lines.join("\n") + "\n"
end

destination.mkpath
count = 0
items.each do |name, item|
  key = item.fetch("key")
  position = item.fetch("position")
  field_count = menus.fetch(name).fetch("options").length
  (destination / "#{set}--verify--#{key}.toml").write(toml(set, name, item, nil, actions_for(set, name, position, field_count, nil)))
  item.fetch("observed_values").each_index do |step|
    stem = "#{set}--#{key}--step-#{format('%02d', step)}"
    (destination / "#{stem}.toml").write(toml(set, name, item, step, actions_for(set, name, position, field_count, step)))
    count += 1
  end
end
puts "#{set}: wrote #{count} isolated samples and #{items.length} fresh-load checks to #{destination}"
