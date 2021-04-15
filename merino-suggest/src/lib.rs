use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Suggestion {
    url: String,
}

pub trait Suggester {
    fn suggest(query: &str) -> Vec<Suggestion>;
}

pub struct WikiFruit;

impl Suggester for WikiFruit {
    fn suggest(query: &str) -> Vec<Suggestion> {
        let url = match query {
            "apple" => Some("https://en.wikipedia.org/wiki/Apple"),
            "banana" => Some("https://en.wikipedia.org/wiki/Banana"),
            "cherry" => Some("https://en.wikipedia.org/wiki/Cherry"),
            _ => None,
        };
        if let Some(url) = url {
            vec![Suggestion {
                url: url.to_string(),
            }]
        } else {
            vec![]
        }
    }
}
