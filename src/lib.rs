use zed_extension_api as zed;

struct AsciiDocExtension;

impl zed::Extension for AsciiDocExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let command = worktree.which("adoc-ls").ok_or_else(|| {
            "adoc-ls was not found on PATH; install it with `cargo install --path crates/adoc-ls`"
                .to_owned()
        })?;

        Ok(zed::Command {
            command,
            args: vec!["--stdio".to_owned()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(AsciiDocExtension);
