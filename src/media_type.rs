pub const OCTET_STREAM: &str = "application/octet-stream";

#[derive(Hash,PartialEq,Debug)]
pub enum AssetClass {
    Image,
    Video,
    Other,
}

#[derive(Hash,PartialEq,Debug)]
pub struct Mime {
    pub mime: String,
    pub class: AssetClass,
}

impl Mime {
    pub fn from_mime_str(mime: &str) -> Mime {
        let r#type = if mime.starts_with("image") {
            // "image/svg+xml" => (),
            // "image/png" => (),
            // "image/jpeg" => (),
            AssetClass::Image
        } else {
            AssetClass::Other
        };
        Mime { mime: mime.to_string(), class: r#type }
    }

    pub fn str(&self) -> &str {
        &self.mime
    }
}

impl Default for Mime {
    fn default() -> Self {
        Self { mime: OCTET_STREAM.to_string(), class: AssetClass::Other }
    }
}