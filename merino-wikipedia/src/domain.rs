//! Data types used to define the data this crate works with;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TryFromInto};

/// A page on Wikipedia. This type is a partial
#[serde_as]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct WikipediaDocument {
    /// The human readable title of the page.
    pub title: String,

    /// The contents of the page, in Wikitext.
    pub page_text: String,

    /// The Wikipedia namespace of the page.
    #[serde_as(as = "TryFromInto<i32>")]
    pub namespace: WikipediaNamespace,

    /// The Wikipedia ID for this page. It is expected that if two pages have
    /// the same page_id, they are the same page, with possible edits.
    pub page_id: u32,
}

/// The namespace of a Wikipedia page. Included here for completeness, but all
/// of the content we are interested are likely in namespace 0, Articles. This
/// will be included in the Wikipedia data that we have access to, and is
/// important to disambiguate page IDs.
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug)]
pub enum WikipediaNamespace {
    Article,
    ArticleTalk,
    User,
    UserTalk,
    Wikipedia,
    WikipediaTalk,
    File,
    FileTalk,
    MediaWiki,
    MediaWikiTalk,
    Template,
    TemplateTalk,
    Help,
    HelpTalk,
    Category,
    CategoryTalk,
    Portal,
    PortalTalk,
    Draft,
    DraftTalk,
    TimedText,
    TimedTextTalk,
    Module,
    ModuleTalk,
    Gadget,
    GadgetTalk,
    GadgetDefinition,
    GadgetDefinitionTalk,
    Special,
    Media,
}

impl TryFrom<i32> for WikipediaNamespace {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Article),
            1 => Ok(Self::ArticleTalk),
            2 => Ok(Self::User),
            3 => Ok(Self::UserTalk),
            4 => Ok(Self::Wikipedia),
            5 => Ok(Self::WikipediaTalk),
            6 => Ok(Self::File),
            7 => Ok(Self::FileTalk),
            8 => Ok(Self::MediaWiki),
            9 => Ok(Self::MediaWikiTalk),
            10 => Ok(Self::Template),
            11 => Ok(Self::TemplateTalk),
            12 => Ok(Self::Help),
            13 => Ok(Self::HelpTalk),
            14 => Ok(Self::Category),
            15 => Ok(Self::CategoryTalk),
            100 => Ok(Self::Portal),
            101 => Ok(Self::PortalTalk),
            118 => Ok(Self::Draft),
            119 => Ok(Self::DraftTalk),
            710 => Ok(Self::TimedText),
            711 => Ok(Self::TimedTextTalk),
            828 => Ok(Self::Module),
            829 => Ok(Self::ModuleTalk),
            2300 => Ok(Self::Gadget),
            2301 => Ok(Self::GadgetTalk),
            2302 => Ok(Self::GadgetDefinition),
            2303 => Ok(Self::GadgetDefinitionTalk),
            -1 => Ok(Self::Special),
            -2 => Ok(Self::Media),
            _ => Err(anyhow!("Unexpected namespace ID {value}")),
        }
    }
}

impl From<WikipediaNamespace> for i32 {
    fn from(ns: WikipediaNamespace) -> Self {
        match ns {
            WikipediaNamespace::Article => 0,
            WikipediaNamespace::ArticleTalk => 1,
            WikipediaNamespace::User => 2,
            WikipediaNamespace::UserTalk => 3,
            WikipediaNamespace::Wikipedia => 4,
            WikipediaNamespace::WikipediaTalk => 5,
            WikipediaNamespace::File => 6,
            WikipediaNamespace::FileTalk => 7,
            WikipediaNamespace::MediaWiki => 8,
            WikipediaNamespace::MediaWikiTalk => 9,
            WikipediaNamespace::Template => 10,
            WikipediaNamespace::TemplateTalk => 11,
            WikipediaNamespace::Help => 12,
            WikipediaNamespace::HelpTalk => 13,
            WikipediaNamespace::Category => 14,
            WikipediaNamespace::CategoryTalk => 15,
            WikipediaNamespace::Portal => 100,
            WikipediaNamespace::PortalTalk => 101,
            WikipediaNamespace::Draft => 118,
            WikipediaNamespace::DraftTalk => 119,
            WikipediaNamespace::TimedText => 710,
            WikipediaNamespace::TimedTextTalk => 711,
            WikipediaNamespace::Module => 828,
            WikipediaNamespace::ModuleTalk => 829,
            WikipediaNamespace::Gadget => 2300,
            WikipediaNamespace::GadgetTalk => 2301,
            WikipediaNamespace::GadgetDefinition => 2302,
            WikipediaNamespace::GadgetDefinitionTalk => 2303,
            WikipediaNamespace::Special => -1,
            WikipediaNamespace::Media => -2,
        }
    }
}
