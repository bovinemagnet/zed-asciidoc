use zed_extension_api as zed;

struct AsciiDocExtension;

impl zed::Extension for AsciiDocExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(AsciiDocExtension);
