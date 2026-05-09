/// How to derive the dlog tag from a `log::Record`.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TagStrategy {
    /// Use `record.target()` if it looks like an explicit override (does not contain `::`),
    /// otherwise fall back to the configured default tag.
    ///
    /// The `log` crate sets `target` to the module path by default, so a value containing
    /// `::` is almost certainly the auto-generated module path rather than a user override.
    #[default]
    TargetThenDefault,

    /// Always use the configured default tag, ignoring `record.target()`.
    AlwaysDefault,

    /// Always use `record.target()` as the dlog tag (module path when not overridden).
    AlwaysTarget,
}

pub(crate) fn resolve<'a>(strategy: TagStrategy, target: &'a str, default_tag: &'a str) -> &'a str {
    match strategy {
        TagStrategy::AlwaysDefault => default_tag,
        TagStrategy::AlwaysTarget => target,
        TagStrategy::TargetThenDefault => {
            if target.is_empty() || target.contains("::") {
                default_tag
            } else {
                target
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_then_default_uses_target_when_flat() {
        assert_eq!(
            resolve(TagStrategy::TargetThenDefault, "Network", "App"),
            "Network"
        );
    }

    #[test]
    fn target_then_default_falls_back_when_module_path() {
        assert_eq!(
            resolve(TagStrategy::TargetThenDefault, "myapp::network", "App"),
            "App"
        );
    }

    #[test]
    fn target_then_default_falls_back_when_empty() {
        assert_eq!(resolve(TagStrategy::TargetThenDefault, "", "App"), "App");
    }

    #[test]
    fn always_default_ignores_target() {
        assert_eq!(resolve(TagStrategy::AlwaysDefault, "Network", "App"), "App");
    }

    #[test]
    fn always_target_uses_target_even_when_module_path() {
        assert_eq!(
            resolve(TagStrategy::AlwaysTarget, "myapp::network", "App"),
            "myapp::network"
        );
    }
}
