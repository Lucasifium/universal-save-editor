//! Wasteland 2 (and Director's Cut) saves: `<name>.xml` in a folder under `Save Games`. It is
//! XML except that character names are written as `<name><@>Max{M}</name>`; those three
//! characters are read as `[@]` (same length) so the rest opens as XML, and edits go into the
//! file as it was.

use super::xml;
use crate::tree::TreeDoc;

pub const KIND: &str = "Wasteland 2 save";

pub fn looks(text: &str) -> bool {
    text.starts_with("<save><version>") && text.contains("<whichScene>") && text.contains("<pcs>")
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not a Wasteland 2 save".into());
    }
    let fixed = text.replace("<@>", "[@]");
    let mut doc = xml::parse_with(&fixed, text.as_bytes(), &["pc"])?;
    doc.kind_name = KIND.into();
    // Names keep their `<@>` marker in the file; they are shown but not edited.
    let view = doc.view.clone();
    doc.editable.retain(|p, _| !view.pointer(p).and_then(|v| v.as_str()).is_some_and(|t| t.starts_with("[@]")));
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "<save><version>0.1</version><whichScene>AZ00</whichScene><pcs><pc><name><@>Max{M}</name><level>1</level><xp>29</xp></pc></pcs></save>";

    #[test]
    fn reads_and_edits_a_ranger() {
        let mut doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/save/pcs/pc/0/xp").unwrap(), 29);
        doc.set("/save/pcs/pc/0/xp", 500.into()).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, SAMPLE.replace(">29<", ">500<"));
    }
}
