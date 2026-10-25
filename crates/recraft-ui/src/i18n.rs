// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Fluent-backed UI localization (English + Simplified Chinese).
//!
//! The FTL resources are embedded at compile time; `I18n` keeps one parsed bundle per
//! locale and falls back to the other locale (then the message key) when a message is
//! missing or fails to format. Callers use [`I18n::text`] rather than the old inline
//! `t(en, zh)` pairs.

use std::sync::Arc;

use fluent::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Zh,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Lang::En => Lang::Zh,
            Lang::Zh => Lang::En,
        }
    }

    const fn index(self) -> usize {
        match self {
            Lang::En => 0,
            Lang::Zh => 1,
        }
    }

    fn langid(self) -> LanguageIdentifier {
        match self {
            Lang::En => "en-US".parse().expect("valid en-US"),
            Lang::Zh => "zh-CN".parse().expect("valid zh-CN"),
        }
    }
}

const EN_FTL: &str = include_str!("../locales/en-US.ftl");
const ZH_FTL: &str = include_str!("../locales/zh-CN.ftl");

/// Two parsed Fluent bundles and the currently selected locale.
type Bundle = FluentBundle<Arc<FluentResource>>;

pub struct I18n {
    bundles: [Bundle; 2],
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        Self {
            bundles: [bundle_for(Lang::En, EN_FTL), bundle_for(Lang::Zh, ZH_FTL)],
        }
    }

    /// Format `key` in `lang`; falls back to the other locale, then to the key itself.
    pub fn text(&self, lang: Lang, key: &str) -> String {
        self.format(lang, key, None)
            .or_else(|| self.format(lang.toggle(), key, None))
            .unwrap_or_else(|| key.to_owned())
    }

    /// Format `key` with named arguments (Fluent `{ $name }` placeholders).
    pub fn text_args(&self, lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, FluentValue::from(*value));
        }
        self.format(lang, key, Some(&fluent_args))
            .or_else(|| self.format(lang.toggle(), key, Some(&fluent_args)))
            .unwrap_or_else(|| key.to_owned())
    }

    fn format(&self, lang: Lang, key: &str, args: Option<&FluentArgs<'_>>) -> Option<String> {
        let bundle = &self.bundles[lang.index()];
        let message = bundle.get_message(key)?;
        let value = message.value()?.clone();
        let mut errors = Vec::new();
        let formatted = bundle.format_pattern(&value, args, &mut errors).to_string();
        Some(formatted)
    }
}

fn bundle_for(lang: Lang, source: &'static str) -> Bundle {
    let resource = FluentResource::try_new(source.to_owned())
        .unwrap_or_else(|(_, errors)| panic!("{lang:?} FTL parse failed: {errors:?}"));
    let mut bundle = FluentBundle::new(vec![lang.langid()]);
    bundle
        .add_resource(Arc::new(resource))
        .unwrap_or_else(|errors| panic!("{lang:?} FTL resource add failed: {errors:?}"));
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_between_two_languages() {
        assert_eq!(Lang::En.toggle(), Lang::Zh);
        assert_eq!(Lang::Zh.toggle(), Lang::En);
    }

    #[test]
    fn formats_known_message_in_both_locales() {
        let i18n = I18n::new();
        let en = i18n.text(Lang::En, "ui_open_project");
        let zh = i18n.text(Lang::Zh, "ui_open_project");
        assert!(!en.is_empty());
        assert!(!zh.is_empty());
        assert_ne!(en, "ui_open_project");
        assert_ne!(zh, "ui_open_project");
    }

    #[test]
    fn missing_message_falls_back_to_key() {
        let i18n = I18n::new();
        assert_eq!(
            i18n.text(Lang::En, "definitely_missing"),
            "definitely_missing"
        );
    }

    #[test]
    fn both_locales_define_the_same_message_ids() {
        fn ids(source: &str) -> std::collections::BTreeSet<&str> {
            source
                .lines()
                .filter(|line| !line.starts_with('#') && line.contains(" = "))
                .filter_map(|line| line.split_once(" = ").map(|(id, _)| id))
                .collect()
        }
        assert_eq!(ids(EN_FTL), ids(ZH_FTL));
    }

    #[test]
    fn formats_arguments_in_the_selected_locale() {
        let i18n = I18n::new();
        let en = i18n.text_args(
            Lang::En,
            "ui_query_matches",
            &[("intent", "strings"), ("count", "3"), ("truncated", "")],
        );
        let zh = i18n.text_args(
            Lang::Zh,
            "ui_query_matches",
            &[("intent", "strings"), ("count", "3"), ("truncated", "")],
        );
        assert!(en.contains("strings") && en.contains('3'), "{en}");
        assert!(zh.contains("strings") && zh.contains('3'), "{zh}");
        assert_ne!(en, zh);
    }
}
