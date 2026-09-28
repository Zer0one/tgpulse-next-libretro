//! Command line parsing.
//!
//! The emulator is usable two ways and this decides which: name a romset and it
//! boots straight into it, name nothing and it opens the library window. Either
//! way the same settings apply. GUI startup flags only open existing panels;
//! `--debug` instead selects the headless scriptable debugger.

use std::path::PathBuf;

use tgpulse_core::config::Config;
use tgpulse_core::library;

use crate::gui::StartupPanels;

/// What the front end should do once arguments are understood.
pub enum Command {
    /// Open the window; boot straight into a romset if one was named.
    Run {
        rom: Option<PathBuf>,
        panels: StartupPanels,
    },
    /// Print the contents of the ROM directory and exit.
    ListRoms,
    /// Run the scriptable debugger over a romset.
    Debug { rom: PathBuf, script: Script },
    /// Print a message and exit successfully (`--help`, `--version`).
    Message(String),
}

/// Where the debugger reads its commands from.
pub enum Script {
    Inline(Vec<String>),
    File(PathBuf),
    Stdin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_uses_the_public_release_tag_when_provided_at_build_time() {
        let args = parse_from(vec!["--version".into()], Config::default()).unwrap();
        let expected = option_env!("TGPULSE_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
        assert!(
            matches!(args.command, Command::Message(text) if text == format!("tgpulse {expected}"))
        );
    }

    #[test]
    fn profiles_resolve_explicit_paths_from_invocation_and_cli_wins() {
        let dir = std::env::temp_dir().join(format!("tgpulse-cli-profile-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("profiles")).unwrap();
        std::fs::create_dir_all(dir.join("sets")).unwrap();
        std::fs::write(dir.join("sets/vr.zip"), []).unwrap();
        std::fs::write(
            dir.join("profiles/master.conf"),
            "volume = 20\nnvram = saves/master.nv\nmodel1_port_in = 25001\n",
        )
        .unwrap();
        let args = [
            "--volume",
            "75",
            "--config",
            "profiles/master.conf",
            "--roms",
            "sets",
            "vr",
        ];
        let parsed = parse_profile(
            args.map(String::from).to_vec(),
            dir.clone(),
            dir.join("unused.conf"),
        )
        .unwrap();
        assert_eq!(parsed.config.volume, 75);
        assert_eq!(parsed.config.rom_dir, dir.join("sets"));
        assert_eq!(parsed.profile.path, dir.join("profiles/master.conf"));
        assert_eq!(parsed.profile.settings.network.port_in, 25001);
        assert_eq!(
            parsed.profile.nvram_file().unwrap().0,
            dir.join("saves/master.nv")
        );
        assert!(
            matches!(parsed.command, Command::Run { rom: Some(p), .. } if p == dir.join("sets/vr.zip"))
        );
        let direct = parse_from_at(vec!["sets/vr.zip".into()], Config::default(), &dir).unwrap();
        assert!(
            matches!(direct.command, Command::Run { rom: Some(p), .. } if p == dir.join("sets/vr.zip"))
        );
        let debug = parse_from_at(
            ["--debug", "sets/vr.zip", "-f", "script.txt"]
                .map(String::from)
                .to_vec(),
            Config::default(),
            &dir,
        )
        .unwrap();
        assert!(
            matches!(debug.command, Command::Debug { script: Script::File(p), .. } if p == dir.join("script.txt"))
        );
        for args in [
            vec!["--config", "missing.conf"],
            vec!["--config", "profiles/master.conf"],
            vec![
                "--config",
                "profiles/master.conf",
                "--config",
                "profiles/master.conf",
            ],
        ] {
            assert!(parse_profile(
                args.into_iter().map(String::from).collect(),
                dir.clone(),
                dir.join("unused.conf")
            )
            .is_err());
        }
        assert!(!dir.join("unused.conf").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn gui_panel_flags_are_independent_and_accept_a_rom() {
        // Resolution only requires an existing file; no ROM is loaded here.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        for show_stats in [false, true] {
            for show_debugger in [false, true] {
                for with_rom in [false, true] {
                    let mut args = Vec::new();
                    if show_stats {
                        args.push("--show-stats".into());
                    }
                    if with_rom {
                        args.push(path.to_string_lossy().into_owned());
                    }
                    if show_debugger {
                        args.push("--show-debugger".into());
                    }
                    let parsed = parse_from(args, Config::default()).unwrap();
                    let Command::Run { rom, panels } = parsed.command else {
                        panic!("expected GUI command");
                    };
                    assert_eq!(rom, with_rom.then(|| path.clone()));
                    assert_eq!(
                        panels,
                        StartupPanels {
                            show_stats,
                            show_debugger
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn gui_panel_flags_reject_headless_commands() {
        for flag in ["--show-stats", "--show-debugger"] {
            for command in ["--debug", "--list"] {
                for args in [
                    vec![flag.into(), command.into()],
                    vec![command.into(), flag.into()],
                ] {
                    let Err(error) = parse_from(args, Config::default()) else {
                        panic!("expected incompatible command error");
                    };
                    assert!(error.contains("require the GUI"));
                }
            }
        }
    }

    #[test]
    fn debug_still_selects_headless_script_runner() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let parsed = parse_from(
            vec![
                path.to_string_lossy().into_owned(),
                "--debug".into(),
                "-c".into(),
                "run 1; regs".into(),
            ],
            Config::default(),
        )
        .unwrap();
        let Command::Debug {
            rom,
            script: Script::Inline(commands),
        } = parsed.command
        else {
            panic!("expected headless debugger");
        };
        assert_eq!(rom, path);
        assert_eq!(commands, ["run 1", "regs"]);
    }

    #[test]
    fn widescreen_modes_override_saved_config() {
        use tgpulse_core::config::Widescreen;
        for (text, expected) in [
            ("auto", Widescreen::Auto),
            ("on", Widescreen::On),
            ("off", Widescreen::Off),
        ] {
            let mut base = Config::default();
            base.widescreen = Widescreen::Auto;
            let parsed = parse_from(vec!["--widescreen".into(), text.into()], base).unwrap();
            assert_eq!(parsed.config.widescreen, expected);
        }
        assert!(parse_from(
            vec!["--widescreen".into(), "invalid".into()],
            Config::default()
        )
        .is_err());
    }

    #[test]
    fn fullscreen_cli_overrides_saved_preference() {
        let mut base = Config::default();
        base.fullscreen = true;
        assert!(parse_from(vec![], base.clone()).unwrap().config.fullscreen);
        assert!(
            !parse_from(vec!["--fullscreen".into(), "off".into()], base)
                .unwrap()
                .config
                .fullscreen
        );
    }
}

pub struct Args {
    pub command: Command,
    pub config: Config,
    pub profile: crate::settings::Profile,
}

/// Parses `std::env::args`. The error is a message ready to print.
pub fn parse() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let launch_dir = std::env::var_os("TGPULSE_LAUNCH_DIR")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir().map_err(|e| e.to_string())?);
    if !launch_dir.is_absolute() || !launch_dir.is_dir() {
        return Err("TGPULSE_LAUNCH_DIR must name an existing absolute directory".into());
    }
    parse_profile(args, launch_dir, crate::settings::Settings::path())
}

fn parse_profile(
    args: Vec<String>,
    launch_dir: PathBuf,
    default_path: PathBuf,
) -> Result<Args, String> {
    use crate::settings::{Profile, Settings};
    // Skip values of other options: an inline debugger command named --config
    // must not be mistaken for a profile selector.
    let mut selected = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                let value = args.get(i).ok_or("--config needs a value")?;
                if selected.is_some() {
                    return Err("--config may be specified only once".into());
                }
                selected = Some(launch_dir.join(value));
            }
            "--roms"
            | "--rom"
            | "-c"
            | "-f"
            | "--cabinet"
            | "--ssaa"
            | "--volume"
            | "--rumble"
            | "--smooth-shadows"
            | "--widescreen"
            | "--widescreen-stretch-2d"
            | "--fullscreen" => i += 1,
            _ => {}
        }
        i += 1;
    }
    // The saved adjustments are for the interactive application; the debugger
    // and the listings stay on the shipped defaults so scripts and captures
    // are reproducible no matter what the settings window last did.
    let headless = args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--debug" | "--list" | "--help" | "-h" | "--version" | "-V"
        )
    });
    let mut base = Config::default();
    if selected.is_some() && args.iter().any(|a| a == "--debug") {
        return Err("--config is not supported by the standalone debugger".into());
    }
    let explicit = selected.is_some();
    let path = selected.unwrap_or(default_path);
    let settings = if explicit {
        std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read config {}: {e}", path.display()))?;
        Settings::load(&path)
    } else if !headless {
        Settings::load_or_create(&path)
    } else {
        Settings::default()
    };
    if !headless || explicit {
        settings.apply_to(&mut base);
    }
    let mut parsed = parse_from_at(args, base, &launch_dir)?;
    if settings.nvram.is_some() && matches!(parsed.command, Command::Run { rom: None, .. }) {
        return Err("a profile with nvram requires a romset on the command line".into());
    }
    parsed.profile = Profile {
        path,
        settings,
        launch_dir,
    };
    Ok(parsed)
}

#[cfg(test)]
fn parse_from(args: Vec<String>, config: Config) -> Result<Args, String> {
    parse_from_at(args, config, &std::env::current_dir().unwrap())
}

fn parse_from_at(
    args: Vec<String>,
    mut config: Config,
    launch_dir: &std::path::Path,
) -> Result<Args, String> {
    let mut rom: Option<String> = None;
    let mut list = false;
    let mut debug = false;
    let mut panels = StartupPanels::default();
    let mut script: Option<Script> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        // `next` consumes the following argument, naming the option in the
        // error so a missing value says which option wanted one.
        let next = |i: &mut usize| -> Result<String, String> {
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg {
            "--config" => {
                next(&mut i)?;
            }
            "--roms" => config.rom_dir = launch_dir.join(next(&mut i)?),
            "--rom" => rom = Some(next(&mut i)?),
            "--list" => list = true,
            "--debug" => debug = true,
            "--show-stats" => panels.show_stats = true,
            "--show-debugger" => panels.show_debugger = true,
            "-c" => script = Some(Script::Inline(split_commands(&next(&mut i)?))),
            "-f" => script = Some(Script::File(launch_dir.join(next(&mut i)?))),
            "--cabinet" => config.cabinet = next(&mut i)?.parse()?,
            "--ssaa" => {
                let v = next(&mut i)?;
                config.ssaa = v
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=4).contains(n))
                    .ok_or_else(|| format!("bad --ssaa '{v}' (want 1..4)"))?;
            }
            "--volume" => {
                let v = next(&mut i)?;
                config.volume = v
                    .parse()
                    .map_err(|_| format!("bad --volume '{v}' (want a percentage)"))?;
            }
            "--rumble" => config.rumble = on_off(arg, &next(&mut i)?)?,
            "--smooth-shadows" => config.smooth_shadows = on_off(arg, &next(&mut i)?)?,
            "--widescreen" => config.widescreen = next(&mut i)?.parse()?,
            "--widescreen-stretch-2d" => {
                config.widescreen_stretch_2d = on_off(arg, &next(&mut i)?)?
            }
            "--fullscreen" => config.fullscreen = on_off(arg, &next(&mut i)?)?,
            "--version" | "-V" => {
                // Cargo's package version is upstream's internal 0.1.0; tagged
                // fork releases pass their public version at build time.
                let version =
                    option_env!("TGPULSE_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
                return Ok(Args {
                    command: Command::Message(format!("tgpulse {version}")),
                    profile: Default::default(),
                    config,
                });
            }
            "--help" | "-h" => {
                return Ok(Args {
                    command: Command::Message(HELP.to_string()),
                    profile: Default::default(),
                    config,
                })
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'\n\n{HELP}"))
            }
            other => rom = Some(other.to_string()),
        }
        i += 1;
    }

    if (list || debug) && (panels.show_stats || panels.show_debugger) {
        return Err("--show-stats and --show-debugger require the GUI; cannot combine with --debug or --list".into());
    }

    if list {
        return Ok(Args {
            command: Command::ListRoms,
            profile: Default::default(),
            config,
        });
    }

    let rom = match rom {
        Some(r) => Some(resolve(&config, &r, launch_dir)?),
        None => None,
    };

    let command = if debug {
        let rom = rom.ok_or("--debug needs a romset")?;
        Command::Debug {
            rom,
            script: script.unwrap_or(Script::Stdin),
        }
    } else {
        Command::Run { rom, panels }
    };

    Ok(Args {
        command,
        config,
        profile: Default::default(),
    })
}

/// Accepts either a path to an archive or a short set name to look up in the
/// ROM directory, so `tgpulse vf2` works as well as `tgpulse roms/vf2.zip`.
fn resolve(config: &Config, arg: &str, launch_dir: &std::path::Path) -> Result<PathBuf, String> {
    let direct = launch_dir.join(arg);
    if direct.is_file() {
        return Ok(direct);
    }
    if arg.ends_with(".zip") || std::path::Path::new(arg).components().count() > 1 {
        return Err(format!("ROM file not found: {}", direct.display()));
    }
    let in_dir = config.rom_dir.join(format!("{arg}.zip"));
    if in_dir.is_file() {
        return Ok(in_dir);
    }
    if let Some(entry) = library::find(&config.rom_dir, arg) {
        return Ok(entry.path);
    }
    Err(format!(
        "no romset '{arg}': not a file, and not in {}",
        config.rom_dir.display()
    ))
}

fn on_off(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("bad {name} '{other}' (want on|off)")),
    }
}

/// Splits `-c "run 60; regs"` into separate commands.
fn split_commands(s: &str) -> Vec<String> {
    s.split(';').map(|c| c.trim().to_string()).collect()
}

/// Prints the ROM directory as a table.
pub fn list_roms(config: &Config) -> String {
    let entries = library::scan(&config.rom_dir);
    if entries.is_empty() {
        return format!(
            "No romsets in {}.\nPut zipped romsets there and try again.",
            config.rom_dir.display()
        );
    }
    let width = entries.iter().map(|e| e.set.len()).max().unwrap_or(8);
    let mut out = format!(
        "{} romset(s) in {}\n",
        entries.len(),
        config.rom_dir.display()
    );
    for e in &entries {
        let board = e.board.map(|b| b.label()).unwrap_or("-");
        out += &format!(
            "  {:<width$}  {:<8}  {:<4}  {}  [{}]\n",
            e.set,
            board,
            e.year,
            e.title,
            e.status(),
        );
    }
    out
}

pub const HELP: &str = "\
TGPulse - a Sega Model 1 and Model 2 arcade emulator

Usage:
  tgpulse                       Open the ROM library
  tgpulse <set|path>            Boot a romset straight away
  tgpulse --list                List the ROM directory and exit
  tgpulse <set> --debug [-c ..] Run the scriptable debugger

ROMs:
  --roms <dir>          Where romsets live (default: roms)
  --config <file>       Settings profile (relative to launch directory)

Video:
  --ssaa 1..4           Supersamples per output pixel on the 3D layer
                        (default 2). The board itself draws without
                        antialiasing; 1 reproduces that exactly.
  --widescreen on|off|auto   Auto follows supported games' NVRAM monitor settings.
                       On renders with a widened field of view rather
                        than stretching the 4:3 image (default off).
  --widescreen-stretch-2d on|off
                        Stretch the 2D tile layers to fill the widened
                        frame (default on).
  --smooth-shadows on|off
                        Blend the hardware's stipple transparency instead
                        of reproducing its checkerboard (default on).
  --fullscreen on|off   Fullscreen during games; library stays windowed (default off).

Audio:
  --volume <pct>        Output volume, as a percentage of the board's own
                        level (default 100). SCSP titles mix quiet.

Machine:
  --cabinet twin|single Model 1/2 network board fitted/absent (default single).
                        Model 1 twin enables TCP; roles remain in NVRAM.

GUI panels (this launch only):
  --show-stats          Open View > Statistics at startup
  --show-debugger       Open View > Debugger at startup
                        May be combined; not available with --debug or --list.

Headless debugger (--debug, no window):
  -c \"cmd; cmd\"         Run these commands, then exit
  -f <file>             Run a command script, then exit
                        With neither, commands are read from stdin.

  -h, --help            Show this help
  -V, --version         Show the version
";
