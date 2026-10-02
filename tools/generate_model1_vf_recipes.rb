#!/usr/bin/env ruby
# frozen_string_literal: true

# Expand the screenshot-verified VF menus into isolated, committed ABI captures.
# Output contains local recipes only; Save RAM belongs outside Git.
require "json"
require "pathname"
require "yaml"

root = Pathname.new(__dir__).parent
output = ARGV.shift
abort "Usage: ruby #{File.basename(__FILE__)} /absolute/output-dir [field-key]" unless
  output && Pathname.new(output).absolute? && ARGV.length <= 1
field_filter = ARGV.shift
destination = Pathname.new(output).expand_path
abort "Output must be outside the repository" if destination == root || destination.to_s.start_with?(root.to_s + "/")
abort "Output already exists" if destination.exist?

doc = YAML.safe_load((root / "data/diagnostic-menus/vf.yaml").read,
                     permitted_classes: [], aliases: true)
abort "YAML set mismatch" unless doc.dig("game", "set") == "vf"
menus = doc.fetch("menus")
abort "Unexpected menu inventory" unless menus.keys == %w[game_assignments coin_assignments manual_setting]
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
    image = root / "docs/diagnostic-evidence/vf/#{item.fetch('key').tr('_', '-')}-#{step}.png"
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

def enter_menu(actions, name)
  frame(actions, 1200)
  frame(actions, 60, "L3")
  frame(actions, 60)
  (name == "game_assignments" ? 5 : 6).times { pulse(actions, "R3") }
  pulse(actions, "L3", 60)
  return unless name == "manual_setting"

  3.times { pulse(actions, "R3") } # MANUAL SETTING on the Coin page
  pulse(actions, "L3", 60)
end

def navigate(actions, name, position)
  steps = name == "game_assignments" ? position - 1 : position
  steps.times { pulse(actions, "R3") }
end

def recipe_actions(name, position, step)
  actions = []
  enter_menu(actions, name)
  navigate(actions, name, position)
  step.times { pulse(actions, "L3", 30) }
  actions << {"capture" => "selected"}
  exit_position = {"game_assignments" => 9, "coin_assignments" => 4, "manual_setting" => 5}.fetch(name)
  (exit_position - position).times { pulse(actions, "R3") }
  pulse(actions, "L3", 60)
  pulse(actions, "L3", 60) if name == "manual_setting" # Coin EXIT commits the EEPROM
  actions << {"capture" => "saved"}
  actions
end

def reload_actions(name, position)
  actions = []
  enter_menu(actions, name)
  navigate(actions, name, position)
  actions << {"capture" => "reloaded"}
  actions
end

def toml(name, item, step, actions)
  lines = ["set = \"vf\"", "menu = #{name.to_json}", "field = #{item.fetch('key').to_json}"]
  if step
    lines += ["expected_value = #{item.fetch('observed_values')[step].to_s.to_json}", "value_step = #{step}"]
  end
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
  (destination / "vf--verify--#{key}.toml").write(toml(name, item, nil, reload_actions(name, item.fetch("position"))))
  item.fetch("observed_values").each_index do |step|
    stem = "vf--#{key}--step-#{format('%02d', step)}"
    (destination / "#{stem}.toml").write(toml(name, item, step, recipe_actions(name, item.fetch("position"), step)))
    count += 1
  end
end
puts "vf: wrote #{count} isolated samples and #{items.length} fresh-load checks to #{destination}"
