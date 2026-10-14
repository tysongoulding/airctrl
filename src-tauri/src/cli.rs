//! Command-line argument parser for airctrl desktop shell and remote control.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CliOptions {
    pub toggle_transcription: bool,
    pub toggle_post_process: bool,
    pub cancel: bool,
    pub start_hidden: bool,
    pub no_tray: bool,
    pub debug: bool,
    pub model: Option<String>,
    pub help: bool,
}

impl CliOptions {
    pub fn parse<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut opts = Self::default();
        let items: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();

        let mut i = 0;
        while i < items.len() {
            match items[i].as_str() {
                "--toggle-transcription" => opts.toggle_transcription = true,
                "--toggle-post-process" => opts.toggle_post_process = true,
                "--cancel" => opts.cancel = true,
                "--start-hidden" => opts.start_hidden = true,
                "--no-tray" => opts.no_tray = true,
                "--debug" => opts.debug = true,
                "--model" => {
                    if i + 1 < items.len() {
                        i += 1;
                        opts.model = Some(items[i].clone());
                    }
                }
                "--help" | "-h" => opts.help = true,
                _ => {}
            }
            i += 1;
        }

        opts
    }

    pub fn is_remote_command(&self) -> bool {
        self.toggle_transcription || self.toggle_post_process || self.cancel
    }

    pub fn print_help() {
        println!(
            "airctrl v0.1.0 — Hands-free Spatial Desktop Automation Engine\n\n\
USAGE:\n    airctrl [OPTIONS]\n\n\
OPTIONS:\n    --toggle-transcription    Toggle speech-to-text recording on active instance\n    \
--toggle-post-process     Toggle dictionary & formatting post-processing\n    \
--cancel                  Immediately cancel active recording and discard audio buffer\n    \
--start-hidden            Launch application minimized without opening main window\n    \
--no-tray                 Launch application without initializing system tray icon\n    \
--debug                   Enable verbose debug diagnostics and open debug drawer\n    \
--model <NAME>            Specify offline speech model to load on startup\n    \
-h, --help                Print this help information\n"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_empty() {
        let empty: Vec<&str> = vec![];
        let opts = CliOptions::parse(empty);
        assert_eq!(opts, CliOptions::default());
        assert!(!opts.is_remote_command());
    }

    #[test]
    fn test_cli_parse_all_flags() {
        let args = vec![
            "--toggle-transcription",
            "--toggle-post-process",
            "--cancel",
            "--start-hidden",
            "--no-tray",
            "--debug",
            "--model",
            "whisper-turbo.en",
            "--help",
        ];
        let opts = CliOptions::parse(args);
        assert!(opts.toggle_transcription);
        assert!(opts.toggle_post_process);
        assert!(opts.cancel);
        assert!(opts.start_hidden);
        assert!(opts.no_tray);
        assert!(opts.debug);
        assert_eq!(opts.model.as_deref(), Some("whisper-turbo.en"));
        assert!(opts.help);
        assert!(opts.is_remote_command());
    }

    #[test]
    fn test_cli_is_remote_command() {
        let opts_remote = CliOptions::parse(vec!["--toggle-transcription"]);
        assert!(opts_remote.is_remote_command());

        let opts_daemon = CliOptions::parse(vec!["--start-hidden", "--debug"]);
        assert!(!opts_daemon.is_remote_command());
    }
}
