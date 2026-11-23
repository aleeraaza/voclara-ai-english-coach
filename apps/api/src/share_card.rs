//! Shareable result image (PNG) for a graded session.
//!
//! Drawn as SVG and rasterised with resvg, using the Plus Jakarta Sans font
//! embedded from `assets/fonts` (SIL OFL, see `assets/fonts/OFL.txt`). No
//! system fonts are read, so the card looks the same on every server.
//! Colours follow the app's "Indus & Marigold" palette (apps/mobile/DESIGN.md).

use std::sync::Arc;

use anyhow::{Context, Result};
use resvg::{tiny_skia, usvg};

const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/PlusJakartaSans_700Bold.ttf");
const FONT_EXTRABOLD: &[u8] = include_bytes!("../assets/fonts/PlusJakartaSans_800ExtraBold.ttf");

/// 4:5 portrait: fits WhatsApp, Instagram and status updates without cropping.
pub const WIDTH: u32 = 1080;
pub const HEIGHT: u32 = 1350;

pub struct CardData<'a> {
    /// "IELTS-style mock" or "Speaking practice".
    pub title: &'a str,
    pub overall: f64,
    pub fluency: f64,
    pub lexical: f64,
    pub grammar: f64,
    pub pronunciation: Option<f64>,
    /// Mock: the cue card theme (e.g. "Travel").
    pub topic: Option<&'a str>,
    /// Demo grades say so on the card.
    pub demo: bool,
    /// Store link printed at the bottom.
    pub store_url: &'a str,
}

/// Fonts are parsed once and shared by every render.
pub struct ShareCards {
    options: usvg::Options<'static>,
}

impl ShareCards {
    pub fn new() -> Result<Self> {
        let mut options = usvg::Options {
            font_family: "Plus Jakarta Sans".into(),
            ..usvg::Options::default()
        };
        let db = Arc::make_mut(&mut options.fontdb);
        db.load_font_data(FONT_BOLD.to_vec());
        db.load_font_data(FONT_EXTRABOLD.to_vec());
        if db.faces().count() < 2 {
            anyhow::bail!("share card fonts failed to load");
        }
        Ok(Self { options })
    }

    pub fn render_png(&self, data: &CardData<'_>) -> Result<Vec<u8>> {
        let svg = svg(data);
        let tree = usvg::Tree::from_str(&svg, &self.options).context("parsing share card SVG")?;
        let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).context("allocating share card")?;
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
        pixmap.encode_png().context("encoding share card PNG")
    }
}

/// Escape text for SVG. Topics and URLs come from content and config, but
/// escaping keeps a stray `&` or `<` from breaking the document.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn band(v: f64) -> String {
    format!("{v:.1}")
}

fn svg(d: &CardData<'_>) -> String {
    // Palette (apps/mobile/src/theme/tokens.ts).
    const INDUS_950: &str = "#031E1C";
    const INDUS_800: &str = "#073F3A";
    const INDUS_600: &str = "#0B7369";
    const INDUS_300: &str = "#6FC4BA";
    const MARIGOLD_500: &str = "#EE9B1F";
    const MARIGOLD_300: &str = "#F8C878";
    const CORAL_500: &str = "#E2554C";
    const MIST: &str = "#E8F1EF";
    const FOG: &str = "#94AAA5";
    const INK: &str = "#2A1900";

    let rows = [
        ("Fluency &amp; coherence", Some(d.fluency)),
        ("Vocabulary", Some(d.lexical)),
        ("Grammar", Some(d.grammar)),
        ("Pronunciation", d.pronunciation),
    ];
    let mut criteria = String::new();
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = 905 + i as i32 * 78;
        let (text, width) = match value {
            Some(v) => (band(*v), (v / 9.0).clamp(0.0, 1.0) * 300.0),
            None => ("–".to_owned(), 0.0),
        };
        criteria.push_str(&format!(
            r#"<text x="120" y="{y}" font-size="32" font-weight="700" fill="{MIST}">{label}</text>
<rect x="540" y="{bar_y}" width="300" height="14" rx="7" fill="{INDUS_800}"/>
<rect x="540" y="{bar_y}" width="{width:.0}" height="14" rx="7" fill="{INDUS_300}"/>
<text x="960" y="{y}" font-size="32" font-weight="800" fill="{MIST}" text-anchor="end">{text}</text>
"#,
            bar_y = y - 12,
        ));
    }

    let topic = d
        .topic
        .map(|t| {
            format!(
                r#"<text x="540" y="842" font-size="34" font-weight="700" fill="{MARIGOLD_300}" text-anchor="middle">Topic: {}</text>"#,
                esc(t)
            )
        })
        .unwrap_or_default();
    let demo = if d.demo {
        format!(
            r#"<rect x="430" y="250" width="220" height="52" rx="26" fill="{CORAL_500}"/>
<text x="540" y="286" font-size="26" font-weight="800" fill="{MIST}" text-anchor="middle">DEMO GRADE</text>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0.4" y2="1">
    <stop offset="0" stop-color="{INDUS_600}"/>
    <stop offset="0.55" stop-color="{INDUS_800}"/>
    <stop offset="1" stop-color="{INDUS_950}"/>
  </linearGradient>
  <radialGradient id="glow" cx="0.5" cy="0.38" r="0.42">
    <stop offset="0" stop-color="{MARIGOLD_500}" stop-opacity="0.35"/>
    <stop offset="1" stop-color="{MARIGOLD_500}" stop-opacity="0"/>
  </radialGradient>
  <linearGradient id="medal" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="{MARIGOLD_300}"/>
    <stop offset="1" stop-color="{MARIGOLD_500}"/>
  </linearGradient>
</defs>
<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#bg)"/>
<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#glow)"/>

<text x="540" y="140" font-size="56" font-weight="800" fill="{MIST}" text-anchor="middle">Voclara</text>
<text x="540" y="200" font-size="30" font-weight="700" fill="{FOG}" text-anchor="middle">{title}</text>
{demo}

<circle cx="540" cy="520" r="190" fill="url(#medal)"/>
<circle cx="540" cy="520" r="160" fill="none" stroke="{INK}" stroke-opacity="0.18" stroke-width="4"/>
<text x="540" y="565" font-size="140" font-weight="800" fill="{INK}" text-anchor="middle">{overall}</text>
<text x="540" y="620" font-size="28" font-weight="700" fill="{INK}" fill-opacity="0.75" text-anchor="middle">overall band</text>
<text x="540" y="785" font-size="36" font-weight="800" fill="{MIST}" text-anchor="middle">My estimated speaking band</text>
{topic}

{criteria}

<text x="540" y="1225" font-size="30" font-weight="800" fill="{MARIGOLD_300}" text-anchor="middle">Practise speaking English on Voclara</text>
<text x="540" y="1268" font-size="24" font-weight="700" fill="{FOG}" text-anchor="middle">{store}</text>
<text x="540" y="1312" font-size="20" font-weight="700" fill="{FOG}" fill-opacity="0.8" text-anchor="middle">IELTS-style practice. Not affiliated with IELTS.</text>
</svg>"#,
        title = esc(d.title),
        overall = band(d.overall),
        store = esc(d.store_url),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data<'a>(topic: Option<&'a str>) -> CardData<'a> {
        CardData {
            title: "IELTS-style mock",
            overall: 6.5,
            fluency: 6.0,
            lexical: 7.0,
            grammar: 6.5,
            pronunciation: None,
            topic,
            demo: true,
            store_url: "https://play.google.com/store/apps/details?id=app.voclara",
        }
    }

    #[test]
    fn renders_a_png_of_the_right_size() {
        let cards = ShareCards::new().unwrap();
        let png = cards.render_png(&data(Some("Travel & <culture>"))).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR width and height, big-endian, right after the signature.
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), WIDTH);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), HEIGHT);
    }

    #[test]
    fn svg_escapes_text_and_shows_bands() {
        let s = svg(&data(Some("A & <B>")));
        assert!(s.contains("Topic: A &amp; &lt;B&gt;"));
        assert!(s.contains(">6.5<"));
        assert!(s.contains("DEMO GRADE"));
        assert!(s.contains("Not affiliated with IELTS"));
        assert!(s.contains("details?id=app.voclara"));
    }
}
