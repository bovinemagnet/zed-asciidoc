use zed_extension_api as zed;

const SERVER_BINARY: &str = "adoc-ls";
const STDIO_ARG: &str = "--stdio";

struct AsciiDocExtension;

fn missing_binary_message() -> String {
    format!(
        "{SERVER_BINARY} was not found on PATH; install it with `cargo install --path crates/adoc-ls`"
    )
}

fn server_command(binary: String, env: Vec<(String, String)>) -> zed::Command {
    zed::Command {
        command: binary,
        args: vec![STDIO_ARG.to_owned()],
        env,
    }
}

impl zed::Extension for AsciiDocExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let binary = worktree
            .which(SERVER_BINARY)
            .ok_or_else(missing_binary_message)?;

        Ok(server_command(binary, worktree.shell_env()))
    }
}

zed::register_extension!(AsciiDocExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_command_requests_stdio_transport() {
        let command = server_command("/usr/local/bin/adoc-ls".to_owned(), Vec::new());

        assert_eq!(command.args, vec!["--stdio".to_owned()]);
    }

    #[test]
    fn server_command_uses_the_resolved_binary_path() {
        let command = server_command("/opt/adoc-ls".to_owned(), Vec::new());

        assert_eq!(command.command, "/opt/adoc-ls");
    }

    #[test]
    fn server_command_forwards_the_shell_environment() {
        let env = vec![("PATH".to_owned(), "/usr/bin".to_owned())];

        let command = server_command("adoc-ls".to_owned(), env.clone());

        assert_eq!(command.env, env);
    }

    #[test]
    fn missing_binary_message_names_the_binary_and_how_to_install_it() {
        let message = missing_binary_message();

        assert!(message.contains(SERVER_BINARY), "{message}");
        assert!(
            message.contains("cargo install --path crates/adoc-ls"),
            "{message}"
        );
    }
}
