// Copyright 2021 Rafael Mardojai CM
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use url::Url;

use super::{Data, Image};

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[derive(Debug, Clone)]
pub enum Social {
    Facebook,
    Mastodon,
    Twitter,
}

#[derive(Debug, Clone)]
pub enum CardSize {
    Small, // Mastodon
    Medium, // Twitter summary
    Large, // Twitter summary with large image || Facebook
}

impl CardSize {
    pub fn image_size(&self) -> (u32, u32) {
        match self {
            Self::Small => (64, 64),
            Self::Medium => (125, 125),
            Self::Large => (500, 250)
        }
    }

    pub fn icon_size(&self) -> i32 {
        match self {
            Self::Small => 32,
            Self::Medium => 48,
            Self::Large => 64
        }
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub title: String,
    pub site: String,
    pub description: Option<String>,
    pub image: Option<Image>,
    pub size: CardSize,
    pub social: Social,
}

impl Card {
    pub fn new(data: &Data, social: Social) -> Result<Card, CardError> {
        //! Create a new Card from the found metadata based on the given Social

        let metadata = data.metadata.clone();
        let mut site = data.url.clone();
        let mut size = CardSize::Large; // Default card size

        // Default meta-tags to lookup the needed values
        let mut title_find = vec_of_strings!["og:title", "twitter:title", "title"];
        let mut description_find = vec_of_strings!["og:description", "twitter:description", "description"];
        let mut image_find = vec_of_strings!["og:image", "twitter:image", "twitter:image:src"];
        let mut type_find = vec_of_strings!["og:type"];

        // Change meta-tags to lookup and default values by the given Social:
        match social {
            Social::Facebook => {
                site = site.to_uppercase();
            },
            Social::Mastodon => {
                image_find = vec_of_strings!["og:image"];
                size = CardSize::Small; // Mastodon always use a small card size

                if metadata.contains_key("og:site_name") {
                    let og_site = metadata.get("og:site_name").unwrap().to_string();
                    if !og_site.is_empty() {
                        site = og_site;
                    }
                }
            },
            Social::Twitter => {
                title_find = vec_of_strings!["twitter:title", "og:title", "title"];
                description_find = vec_of_strings!["twitter:description", "og:description"];
                image_find = vec_of_strings!["twitter:image", "twitter:image:src", "og:image"];
                type_find = vec_of_strings!["twitter:card", "og:type"];

                // Change card size by the value of "twitter:card" meta-tag
                if metadata.contains_key("twitter:card") {
                    match metadata.get("twitter:card").unwrap().as_str() {
                        "summary_large_image" => (), // Do nothing
                        "summary" => size = CardSize::Medium,
                        _ => ()
                    }
                } else {
                    size = CardSize::Medium;
                }
            }
        }

        // Get first available value from meta-tags to lookup
        let pre_title = Card::get_correct_tag(&title_find, &metadata, false);
        let title = match &pre_title {
            Some(title) => title.to_string(),
            None => {
                match &data.title {
                    Some(title) => title.to_string(),
                    None => site.to_string(),
                }
            }
        };
        let description = Card::get_correct_tag(&description_find, &metadata, false);
        // TODO: Get description from HTML for Facebook
        let pre_image = Card::get_correct_tag(&image_find, &metadata, true);
        let mut image = match pre_image { // Convert image String to a Image struct:
            Some(url) => Some(Image::new(url)),
            None => None
        };
        let card_type = Card::get_correct_tag(&type_find, &metadata, false);

        // Return error if no basic data is found
        if let (Social::Twitter, None, None) = (&social, &pre_title, &description) {
            return Err(CardError::NotEnoughData);
        }

        // Final per social media match with obtained results
        match social {
            Social::Facebook => {
                // Facebook: If no image found in metadata, get first document image
                if let None = image {
                    if data.images.len() > 0 {
                        image = Some(data.images[0].clone());
                        size = CardSize::Medium;
                    }
                }
            },
            Social::Mastodon => {},
            Social::Twitter => {
                if let None = card_type {
                    return Err(CardError::TwitterNoCardFound);
                }
            },
        }

        Ok(Card {title, site, description, image, size, social})
    }

    pub fn get_correct_tag(
            list: &Vec<String>,
            metadata: &HashMap<String, String>,
            is_url: bool) -> Option<String> {
        //! Get first available value from meta-tags to lookup

        for term in list.iter() {
            if let Some(content) = metadata.get(term) {
                let content = if is_url {
                    match Url::parse(content.trim()) {
                        Ok(_) => content.clone(),
                        Err(_) => {
                            continue
                        }
                    }
                } else {
                    if !content.is_empty() {
                        content.clone()
                    } else {
                        continue
                    }
                };
                return Some(content);
            }
        }

        None
    }
}

#[derive(Debug)]
pub enum CardError {
    NotEnoughData,
    TwitterNoCardFound
}

impl Display for CardError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            CardError::NotEnoughData => write!(f, "NotEnoughData"),
            CardError::TwitterNoCardFound => write!(f, "TwitterNoCardFound"),
        }
    }
}

impl error::Error for CardError {}
