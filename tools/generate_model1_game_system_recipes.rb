#!/usr/bin/env ruby
# frozen_string_literal: true

# Expand the screenshot-verified VR / Virtua Formula Game System YAML into
# isolated, game-committed direct-ABI recipes. Output belongs outside Git.
require "json"
require "pathname"
require "yaml"

root = Pathname.new(__dir__).parent
set = ARGV.shift
output = ARGV.shift
abort "Usage: ruby #{File.basename(__FILE__)} vr|vformula /absolute/output-dir [field-key]" unless
  %w[vr vformula].include?(set) && output && Pathname.new(output).absolute? && ARGV.length <= 1
field_filter = ARGV.shift
destination = Pathname.new(output).expand_path
abort "Output must be outside the repository" if destination == root || destination.to_s.start_with?(root.to_s + "/")
abort "Output already exists" if destination.exist?

doc = YAML.safe_load((root / "data/diagnostic-menus/#{set}.yaml").read,
                     permitted_classes: [], aliases: true)
abort "YAML set mismatch" unless doc.dig("game", "set") == set
page = doc.dig("menus", "game_system") or abort "Game System inventory missing"
options = page.fetch("options")
abort "Incomplete field positions" unless options.map { |item| item.fetch("position") } == (1..options.length).to_a
abort "Unexpected EXIT position" unless page.fetch("exit_position") == options.length + 1
abort "Value cycle not closed" unless options.all? { |item| item["values_status"] == "cycle_closed" }
abort "Default differs from cycle start" unless options.all? do |item|
  item.fetch("native_default").to_s == item.fetch("observed_values").first.to_s
end
if field_filter
  options = options.select { |item| item.fetch("key") == field_filter }
  abort "Unknown field key" if options.empty?
end

def frame(actions, count, buttons = [])
  actions << {"frames" => count, "buttons" => buttons}
end

def pulse(actions, button, held)
  frame(actions, 10, held + [button])
  frame(actions, 20, held)
end

def recipe_actions(set, position, step, exit_position)
  actions = []
  held = set == "vr" ? ["L3"] : []
  if set == "vr"
    frame(actions, 300, held)
  else
    frame(actions, 1200)
    frame(actions, 60, ["L3"])
    frame(actions, 60)
  end
  2.times { pulse(actions, "B", held) }
  if set == "vformula"
    frame(actions, 10, ["X"])
    frame(actions, 60)
  else
    pulse(actions, "X", held)
  end
  position.times { pulse(actions, "B", held) }
  step.times { pulse(actions, "A", held) }
  actions << {"capture" => "selected"}
  (exit_position - position).times { pulse(actions, "B", held) }
  if set == "vformula"
    frame(actions, 10, ["X"])
    frame(actions, 60)
    pulse(actions, "B", held) # YES (SAVED)
    frame(actions, 10, ["X"])
    frame(actions, 60)
  else
    pulse(actions, "X", held)
    pulse(actions, "B", held) # YES (SAVED)
    pulse(actions, "X", held)
    frame(actions, 60, held)
  end
  actions << {"capture" => "saved"}
  actions
end

def reload_actions(set, position)
  actions = []
  held = set == "vr" ? ["L3"] : []
  if set == "vr"
    frame(actions, 300, held)
  else
    frame(actions, 1200)
    frame(actions, 60, ["L3"])
    frame(actions, 60)
  end
  2.times { pulse(actions, "B", held) }
  if set == "vformula"
    frame(actions, 10, ["X"])
    frame(actions, 60)
  else
    pulse(actions, "X", held)
  end
  position.times { pulse(actions, "B", held) }
  actions << {"capture" => "reloaded"}
  actions
end

def toml(set, item, step, actions)
  lines = ["set = #{set.to_json}", "field = #{item.fetch('key').to_json}",
           "expected_value = #{item.fetch('observed_values')[step].to_s.to_json}",
           "value_step = #{step}", ""]
  actions.each do |action|
    lines << "[[actions]]"
    if action.key?("capture")
      lines << "capture = #{action.fetch('capture').to_json}"
    else
      lines << "frames = #{action.fetch('frames')}"
      lines << "buttons = #{action.fetch('buttons').to_json}" unless action.fetch("buttons").empty?
    end
    lines << ""
  end
  lines.join("\n")
end

destination.mkpath
count = 0
options.each do |item|
  verification = ["set = #{set.to_json}", "field = #{item.fetch('key').to_json}", ""]
  reload_actions(set, item.fetch("position")).each do |action|
    verification << "[[actions]]"
    if action.key?("capture")
      verification << "capture = #{action.fetch('capture').to_json}"
    else
      verification << "frames = #{action.fetch('frames')}"
      verification << "buttons = #{action.fetch('buttons').to_json}" unless action.fetch("buttons").empty?
    end
    verification << ""
  end
  (destination / "#{set}--verify--#{item.fetch('key')}.toml").write(verification.join("\n"))
  item.fetch("observed_values").each_index do |step|
    stem = "#{set}--#{item.fetch('key')}--step-#{format('%02d', step)}"
    actions = recipe_actions(set, item.fetch("position"), step, page.fetch("exit_position"))
    (destination / "#{stem}.toml").write(toml(set, item, step, actions))
    count += 1
  end
end
puts "#{set}: wrote #{count} isolated samples and #{options.length} fresh-load checks to #{destination}"
