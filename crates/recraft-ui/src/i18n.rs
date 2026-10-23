// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Minimal two-language string table (English + Simplified Chinese).

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
}

/// Pick the translation for the current language.
pub fn tr(lang: Lang, en: &'static str, zh: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Zh => zh,
    }
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
    fn picks_the_requested_language() {
        assert_eq!(tr(Lang::En, "Open", "打开"), "Open");
        assert_eq!(tr(Lang::Zh, "Open", "打开"), "打开");
    }
}
