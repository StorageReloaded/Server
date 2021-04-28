pub mod storage_reloaded {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Item {
        id: Option<u64>,
        name: String,
        description: String,
        image: String,
        location: u64,
        tags: Vec<u64>,
        amount: u64,
        properties_internal: Vec<Property>,
        properties_custom: Vec<Property>,
        attachments: HashMap<String, String>,
        last_edited: u64,
        created: u64,
    }

    impl Item {
        pub fn new(
            id: Option<u64>,
            name: String,
            description: String,
            image: String,
            location: u64,
            tags: Vec<u64>,
            amount: u64,
            properties_internal: Vec<Property>,
            properties_custom: Vec<Property>,
            attachments: std::collections::HashMap<String, String>,
            last_edited: u64,
            created: u64,
        ) -> Item {
            Item {
                id,
                name,
                description,
                image,
                location,
                tags,
                amount,
                properties_internal,
                properties_custom,
                attachments,
                last_edited,
                created,
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Property {
        id: u64,
        name: String,
        value: String,
        display_type: Option<String>,
        min: Option<u64>,
        max: Option<u64>,
    }

    impl Property {
        pub fn new(
            id: u64,
            name: String,
            value: String,
            display_type: Option<String>,
            min: Option<u64>,
            max: Option<u64>,
        ) -> Property {
            Property {
                id,
                name,
                value,
                display_type,
                min,
                max
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
