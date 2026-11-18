//! Retro TV design: Home as a stylised 1990s cable-TV info channel. Framed blue panels, bold
//! yellow headings with a heavy black outline, white labels with yellow values, the selected
//! app on a little broadcast monitor, weather only in the bottom ticker, and faint scanlines. Everything is static while idle except the half-second cursor blink and
//! the ticker line, which changes with a hard cut every few seconds.
use super::{Assets, FACE_HEIGHT, Scene, c, text};
use crate::{
    state::{RETRO_ROWS, Rect, State, Target},
    weather,
};
use time::OffsetDateTime;

const NAVY: u32 = 0x070d3a;
const PANEL_TOP: u32 = 0x2a3fb8;
const PANEL_BOTTOM: u32 = 0x14227e;
const FRAME: u32 = 0x9fb2ff;
const YELLOW: u32 = 0xf2cf2e;
const WHITE: u32 = 0xf2f2f2;
const CYAN: u32 = 0x7fd8ff;
const ORANGE: u32 = 0xe8892a;
/// Seconds each ticker message stays before the next one cuts in.
const TICKER_SECONDS: i128 = 8;
/// The tracking wobble shown when Home appears.
const GLITCH_SECONDS: f32 = 0.24;
const LIST_X: f32 = 700.;
const LIST_W: f32 = 1068.;
const ROW_Y: f32 = 236.;
const ROW_PITCH: f32 = 74.;

fn rgb(value: u32) -> [f32; 4] {
    c(value, 1.)
}

fn mix(a: u32, b: u32, t: f32) -> [f32; 4] {
    let (a, b) = (rgb(a), rgb(b));
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.,
    ]
}

/// A framed info panel: a vertical blue gradient, a light bevel frame and a dark inner line.
fn panel(out: &mut Scene, rect: Rect) {
    const BANDS: usize = 12;
    let band = rect.h / BANDS as f32;
    for index in 0..BANDS {
        out.shape(
            Rect::new(rect.x, rect.y + index as f32 * band, rect.w, band + 0.5),
            0.,
            mix(PANEL_TOP, PANEL_BOTTOM, index as f32 / (BANDS - 1) as f32),
        );
    }
    out.border(rect, 0., 3., c(FRAME, 0.9));
    out.border(rect.inset(3.), 0., 2., c(0x000000, 0.45));
}

/// Broadcast lettering: a heavy black outline and drop shadow under the coloured text.
fn outlined(out: &mut Scene, label: &str, rect: Rect, size: f32, color: u32, center: bool) {
    let style = text(size, 800, 0.);
    let w = (size / 16.).clamp(2., 6.);
    for (dx, dy) in [(w + 2., w + 2.), (-w, -w), (w, -w), (-w, w), (w, w)] {
        out.text(
            label,
            Rect::new(rect.x + dx, rect.y + dy, rect.w, rect.h),
            style,
            c(0x000000, 0.95),
            center,
        );
    }
    out.text(label, rect, style, rgb(color), center);
}

/// Smaller lettering with only the hard drop shadow.
fn shadowed(out: &mut Scene, label: &str, rect: Rect, size: f32, color: [f32; 4], right: bool) {
    let style = if right {
        text(size, 800, 0.).right()
    } else {
        text(size, 800, 0.)
    };
    out.text(
        label,
        Rect::new(rect.x + 3., rect.y + 3., rect.w, rect.h),
        style,
        c(0x000000, 0.8),
        false,
    );
    out.text(label, rect, style, color, false);
}

fn upper(value: &str) -> String {
    value.to_uppercase()
}

/// The label and value of one menu row.
fn row_text(state: &State, target: &Target) -> (String, String) {
    let language = state.preferences.language;
    match target {
        Target::App(id) => (
            state
                .inventory
                .apps
                .iter()
                .find(|app| &app.id == id)
                .map_or_else(String::new, |app| app.title.clone()),
            String::new(),
        ),
        Target::Nav(_) => (
            language.t("retroAllApps").into(),
            state.drawer_apps().len().to_string(),
        ),
        Target::InputsMenu => (
            language.t("inputs").into(),
            state
                .inventory
                .inputs
                .iter()
                .filter(|input| state.preferences.input_visible(input))
                .count()
                .to_string(),
        ),
        Target::LumaSettings => (language.t("launcherSettings").into(), "ROBOS".into()),
        Target::TvSettings => (language.t("deviceSettings").into(), "TV".into()),
        _ => (String::new(), String::new()),
    }
}

/// A retro menu page: Home ("Main menu") or All apps, as framed panels with a numbered list.
pub(super) fn page(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    out.shape(Rect::new(0., 0., 1920., 1080.), 0., rgb(NAVY));
    // Faint diagonal broadcast stripes behind the header.
    for index in 0..14 {
        out.shape(
            Rect::new(index as f32 * 150. - 40., 24., 60., 150.),
            0.,
            c(0xffffff, 0.025),
        );
    }

    // Header: only the title; the clock and date sit in the ticker's label block.
    let title_box = Rect::new(96., 36., 1728., 140.);
    panel(out, title_box);
    outlined(
        out,
        &upper(language.t(if state.page == crate::state::Page::Apps {
            "retroAllApps"
        } else {
            "retroMainMenu"
        })),
        Rect::new(132., 66., 1200., 80.),
        64.,
        YELLOW,
        false,
    );
    // Main panel: the selected item on the left, the menu on the right.
    panel(out, Rect::new(96., 200., 1728., 740.));
    preview_box(out, state, assets);
    menu(out, state, assets, now);
    ticker(out, state, assets, now);
    glitch(out, state);
    scanlines(out);
    tv_faults(out, now, state.preferences.reduced_motion);
}

/// The "now selected" box: the focused row's app card or icon, framed like a picture on an
/// old broadcast monitor, with its channel number and name.
fn preview_box(out: &mut Scene, state: &State, assets: &Assets) {
    let frame = Rect::new(150., 252., 470., 620.);
    out.shape(frame, 14., c(0x0b1660, 0.55));
    out.border(frame, 14., 5., rgb(ORANGE));
    let rows = state.retro_rows();
    let Some(index) = rows.iter().position(|row| *row == state.focus) else {
        return;
    };
    let target = &rows[index];
    outlined(
        out,
        &format!("CH {:02}", index % RETRO_ROWS + 1),
        Rect::new(frame.x, frame.y + 28., frame.w, 70.),
        52.,
        YELLOW,
        true,
    );
    let screen = Rect::new(
        frame.x + 35.,
        frame.y + 150.,
        400.,
        400. * FACE_HEIGHT / 270.,
    );
    // Monitor bezel: a heavy black frame with a light bevel, the picture inset behind glass.
    out.shape(screen.inset(-14.), 10., rgb(0x000000));
    out.border(screen.inset(-14.), 10., 3., c(FRAME, 0.9));
    let app = match target {
        Target::App(id) => state.inventory.apps.iter().find(|app| &app.id == id),
        _ => None,
    };
    if let Some(app) = app {
        super::app_face(out, assets, app, screen, state.preferences.language, 6.);
    } else {
        out.shape(screen, 6., mix(PANEL_TOP, PANEL_BOTTOM, 0.5));
        let icon = match target {
            Target::Nav(_) => "grid",
            Target::InputsMenu => "input-hdmi",
            Target::LumaSettings => "luma-mark",
            _ => "tv",
        };
        let size = screen.h * 0.62;
        let rect = Rect::new(
            screen.x + (screen.w - size) / 2.,
            screen.y + (screen.h - size) / 2.,
            size,
            size,
        );
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 5., rect.y + 5., rect.w, rect.h),
            c(0x000000, 0.9),
        );
        out.icon(assets, icon, rect, rgb(YELLOW));
    }
    // CRT picture: denser scanlines and a soft glare across the top of the screen.
    let mut y = screen.y + 1.;
    while y < screen.y + screen.h {
        out.shape(Rect::new(screen.x, y, screen.w, 1.5), 0., c(0x000000, 0.18));
        y += 3.;
    }
    out.shape(
        Rect::new(screen.x, screen.y, screen.w, screen.h * 0.35),
        6.,
        c(0xffffff, 0.05),
    );
    let (label, _) = row_text(state, target);
    let label = upper(&label);
    let (label, style) = assets.fit_line(&label, text(40., 800, 0.), frame.w - 48., 26.);
    out.text(
        label.clone(),
        Rect::new(frame.x + 3., frame.y + 523., frame.w, 56.),
        style,
        c(0x000000, 0.8),
        true,
    );
    out.text(
        label,
        Rect::new(frame.x, frame.y + 520., frame.w, 56.),
        style,
        rgb(WHITE),
        true,
    );
}

/// Whether the blinking cursor is in its visible half-second.
fn cursor_on(now: OffsetDateTime) -> bool {
    (now.unix_timestamp_nanos() / super::RETRO_BLINK_NANOS) % 2 == 0
}

fn menu(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let rows = state.retro_rows();
    let focused = rows.iter().position(|row| *row == state.focus);
    let page = focused.unwrap_or(0) / RETRO_ROWS;
    let pages = rows.len().div_ceil(RETRO_ROWS);
    let first = page * RETRO_ROWS;
    for (slot, target) in rows.iter().enumerate().skip(first).take(RETRO_ROWS) {
        let y = ROW_Y + (slot - first) as f32 * ROW_PITCH;
        let rect = Rect::new(LIST_X, y, LIST_W, ROW_PITCH - 8.);
        let active = Some(slot) == focused;
        let moving = active && state.moving.is_some();
        if active {
            out.shape(rect, 6., c(YELLOW, if moving { 0.32 } else { 0.16 }));
            out.border(rect, 6., 3., rgb(YELLOW));
            if cursor_on(now) || moving {
                shadowed(
                    out,
                    if moving { "↕" } else { "▶" },
                    Rect::new(rect.x - 52., y + 10., 40., 50.),
                    40.,
                    rgb(YELLOW),
                    false,
                );
            }
        }
        let (label, value) = row_text(state, target);
        shadowed(
            out,
            &format!("{:02}", slot - first + 1),
            Rect::new(rect.x + 22., y + 13., 80., 46.),
            38.,
            rgb(CYAN),
            false,
        );
        let label = upper(&label);
        let value_width = if value.is_empty() {
            0.
        } else {
            assets.width(&value, text(38., 800, 0.)) + 40.
        };
        let (label, style) = assets.fit_line(
            &label,
            text(38., 800, 0.),
            rect.w - 140. - value_width - 24.,
            26.,
        );
        out.text(
            label.clone(),
            Rect::new(rect.x + 123., y + 16., rect.w, 46.),
            style,
            c(0x000000, 0.8),
            false,
        );
        out.text(
            label,
            Rect::new(rect.x + 120., y + 13., rect.w, 46.),
            style,
            rgb(if active { YELLOW } else { WHITE }),
            false,
        );
        if !value.is_empty() {
            shadowed(
                out,
                &upper(&value),
                Rect::new(rect.x, y + 13., rect.w - 24., 46.),
                38.,
                rgb(YELLOW),
                true,
            );
        }
        out.control(target.clone(), rect, false);
    }
    if pages > 1 {
        shadowed(
            out,
            &format!("{} / {}", page + 1, pages),
            Rect::new(
                LIST_X,
                ROW_Y + RETRO_ROWS as f32 * ROW_PITCH + 2.,
                LIST_W - 24.,
                36.,
            ),
            28.,
            c(WHITE, 0.75),
            true,
        );
    }
}

fn ticker(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    let bar = Rect::new(0., 964., 1920., 80.);
    out.shape(bar, 0., rgb(0x0a1550));
    out.shape(Rect::new(0., bar.y, 1920., 3.), 0., c(FRAME, 0.9));
    out.shape(
        Rect::new(0., bar.y + bar.h - 3., 1920., 3.),
        0.,
        c(FRAME, 0.9),
    );
    let label = Rect::new(0., bar.y + 3., 460., bar.h - 6.);
    out.shape(label, 0., rgb(ORANGE));
    let clock = upper(&language.clock(
        now,
        state.preferences.hour_24,
        state.preferences.show_seconds,
    ));
    let date = upper(&state.preferences.date_format.format(language, now));
    let clock_style = assets.fit_text(&clock, text(34., 800, 0.), 344.);
    let date_style = assets.fit_text(&date, text(22., 800, 0.), 344.);
    for (value, style, y) in [
        (clock, clock_style, bar.y + 9.),
        (date, date_style, bar.y + 47.),
    ] {
        out.text(
            value.clone(),
            Rect::new(98., y + 2., 344., style.size * 1.3),
            style,
            c(0x000000, 0.8),
            false,
        );
        out.text(
            value,
            Rect::new(96., y, 344., style.size * 1.3),
            style,
            rgb(WHITE),
            false,
        );
    }
    let mut messages = vec![upper(language.t("retroHint"))];
    if let Some(place) = state
        .preferences
        .weather_location
        .as_ref()
        .filter(|_| state.preferences.show_weather)
        && let Some(reading) = state.weather.reading(place)
    {
        messages.push(upper(&format!(
            "{}: {}, {}",
            place.name,
            weather::format_temperature(reading.temperature, state.preferences.temperature_unit),
            language.t(weather::condition(reading).key)
        )));
    }
    let index =
        (now.unix_timestamp_nanos() / (TICKER_SECONDS * 1_000_000_000)) as usize % messages.len();
    shadowed(
        out,
        &messages[index],
        Rect::new(500., bar.y + 18., 1324., 46.),
        36.,
        rgb(WHITE),
        false,
    );
}

/// Bands of tracking noise rolling up once while Home appears; nothing when motion is reduced.
fn glitch(out: &mut Scene, state: &State) {
    let Some(started) = state.page_glide() else {
        return;
    };
    let t = started.elapsed().as_secs_f32() / GLITCH_SECONDS;
    if t >= 1. {
        return;
    }
    let mut seed = 0x9e37_79b9_u32 ^ (t * 1000.) as u32;
    let mut random = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f32 / 1000.
    };
    let center = 1180. - t * 1280.;
    for line in 0..18 {
        let y = center - 70. + line as f32 * 8.;
        let mut x = 0.;
        while x < 1920. {
            let w = 30. + random() * 220.;
            out.shape(
                Rect::new(x, y, w, 4. + random() * 4.),
                0.,
                c(0xffffff, (0.15 + random() * 0.45) * (1. - t * 0.6)),
            );
            x += w + random() * 120.;
        }
    }
}

/// CRT scanlines: one faint dark line every four logical pixels, static.
fn scanlines(out: &mut Scene) {
    let mut y = 2.;
    while y < 1080. {
        out.shape(Rect::new(0., y, 1920., 1.5), 0., c(0x000000, 0.12));
        y += 4.;
    }
}

/// Brightness of a fill, to tell focused (light) glass from ordinary panels.
fn luminance(color: [f32; 4]) -> f32 {
    0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]
}

/// Retro TV look for a Liquid Glass page (the launcher Settings): the page keeps its layout,
/// controls and navigation, while its draws are restyled from `from` on. The wallpaper becomes
/// the navy screen, glass becomes framed blue panels, the focused (light) glass turns yellow,
/// soft shadows and glass rims go, corners square up, and lettering turns bold with a hard
/// drop shadow. Scanlines finish the page.
pub(super) fn restyle(out: &mut Scene, assets: &Assets, from: usize) {
    use crate::renderer::Draw;
    let corner = |radius: f32| radius.min(6.);
    let draws = std::mem::take(&mut out.draw);
    let mut restyled = Vec::with_capacity(draws.len() * 2);
    for (index, draw) in draws.into_iter().enumerate() {
        if index < from {
            restyled.push(draw);
            continue;
        }
        match draw {
            Draw::Image(_, rect, ..) | Draw::TintedImage(_, rect, ..)
                if rect.w >= 1920. && rect.h >= 1080. =>
            {
                restyled.push(Draw::Shape(rect, 0., 0., rgb(NAVY)));
            }
            Draw::Glass(rect, radius, tint) => {
                if luminance(tint) > 0.6 {
                    restyled.push(Draw::Shape(rect, corner(radius), 0., rgb(YELLOW)));
                } else {
                    restyled.push(Draw::RoundedGradient(
                        rect,
                        corner(radius),
                        rgb(PANEL_TOP),
                        rgb(PANEL_BOTTOM),
                    ));
                    restyled.push(Draw::Shape(rect, corner(radius), 2., c(FRAME, 0.85)));
                }
            }
            Draw::Rim(rect, radius, ..) => {
                restyled.push(Draw::Shape(rect, corner(radius), 3., rgb(YELLOW)));
            }
            Draw::Shadow(..) => {}
            Draw::Shape(rect, radius, width, color) => {
                // Light hairlines around glass read as the panel frame already.
                if width > 0. && width < 2. && luminance(color) > 0.8 {
                    continue;
                }
                restyled.push(Draw::Shape(rect, corner(radius), width, color));
            }
            Draw::RoundedGradient(rect, radius, top, bottom) => {
                restyled.push(Draw::RoundedGradient(rect, corner(radius), top, bottom));
            }
            Draw::Text(label, rect, style, color, center) => {
                // Bold lettering is wider: shrink it to the box the page laid out, so
                // nothing that fitted before (the clock, values) gets cut.
                let mut bold = style;
                bold.weight = 800;
                // Only single-line boxes: paragraphs wrap instead.
                let single_line = rect.h < 2. * style.size * style.line_height;
                let style =
                    if single_line && rect.w > 0. && assets.width(&label, bold) > rect.w - 1. {
                        let fitted = assets.fit_text(&label, bold, rect.w - 1.);
                        if assets.width(&label, fitted) <= rect.w - 1. {
                            fitted
                        } else {
                            style
                        }
                    } else {
                        bold
                    };
                if luminance(color) > 0.5 {
                    restyled.push(Draw::Text(
                        label.clone(),
                        Rect::new(rect.x + 2., rect.y + 2., rect.w, rect.h),
                        style,
                        [0., 0., 0., 0.8 * color[3]],
                        center,
                    ));
                }
                restyled.push(Draw::Text(label, rect, style, color, center));
            }
            other => restyled.push(other),
        }
    }
    out.draw = restyled;
    scanlines(out);
}

/// Seconds for the CRT beam to sweep once from the top of the picture to the bottom.
const BEAM_SECONDS: f64 = 6.;
/// Glitches come at most once per window, at a random moment inside it, for a short burst.
const GLITCH_WINDOW: f64 = 12.;
const GLITCH_BURST: f64 = 0.3;

/// SplitMix-style hash: the same window always glitches at the same moment and place.
fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit(value: u64) -> f32 {
    (value % 10_000) as f32 / 10_000.
}

/// Old-TV picture faults over a retro page: a CRT beam with a short afterglow sweeping down the picture and, now
/// and then, a short burst of tracking noise with a brightness flicker. Everything follows the
/// clock, so captures are reproducible; nothing is drawn with Reduce motion. Only these draws
/// change between frames, so partial redraw keeps the per-frame work to the beam's strip.
pub(super) fn tv_faults(out: &mut Scene, now: OffsetDateTime, reduced_motion: bool) {
    if reduced_motion {
        return;
    }
    let seconds = now.unix_timestamp_nanos() as f64 / 1e9;
    // CRT beam: a thin bright scan line sweeping down, with a fading afterglow above it that
    // lights the scanlines it has just passed. Only this strip changes from frame to frame.
    let travel = 1080. + 260.;
    let beam = ((seconds / BEAM_SECONDS).fract() * travel - 40.) as f32;
    const GLOW: usize = 14;
    for strip in 0..GLOW {
        let fade = 1. - strip as f32 / GLOW as f32;
        out.shape(
            Rect::new(0., beam - (strip + 1) as f32 * 12., 1920., 12.5),
            0.,
            c(0xbfd4ff, 0.05 * fade * fade),
        );
    }
    out.shape(Rect::new(0., beam - 1.5, 1920., 3.), 0., c(0xe6efff, 0.16));
    // Occasional tracking burst.
    let window = (seconds / GLITCH_WINDOW).floor();
    let seed = hash(window as u64);
    let start = window * GLITCH_WINDOW + f64::from(unit(seed)) * (GLITCH_WINDOW - 1.);
    let t = seconds - start;
    if !(0. ..GLITCH_BURST).contains(&t) {
        return;
    }
    let mut state = hash(seed ^ (t * 30.) as u64);
    let mut random = move || {
        state = hash(state);
        unit(state)
    };
    // A brief overall flicker, then a band of noisy streaks.
    out.shape(
        Rect::new(0., 0., 1920., 1080.),
        0.,
        c(0xffffff, 0.02 + 0.03 * random()),
    );
    let band = 120. + random() * 760.;
    for line in 0..10 {
        let y = band + line as f32 * 7.;
        let mut x = random() * 200.;
        while x < 1920. {
            let w = 20. + random() * 180.;
            out.shape(
                Rect::new(x, y, w, 2. + random() * 3.),
                0.,
                c(0xffffff, 0.1 + random() * 0.35),
            );
            x += w + random() * 160.;
        }
    }
}

/// The RobOS boot after switching the design: to Liquid Glass a calm glow and the logo on
/// black; to Retro TV a CRT power-on (a line opening into the blue screen) and the logo in
/// broadcast lettering. Drawn over the page; the new design is already live underneath.
pub(super) fn design_boot(out: &mut Scene, assets: &Assets, design: crate::model::Design, t: f32) {
    let ease = |x: f32| {
        let x = x.clamp(0., 1.);
        x * x * (3. - 2. * x)
    };
    let start = out.draw.len();
    let full = Rect::new(0., 0., 1920., 1080.);
    let total = crate::state::boot_seconds(design);
    // Liquid Glass fades out at the end; Retro TV cuts away through a VHS glitch instead.
    let fade_out = match design {
        crate::model::Design::Glass => 1. - ease((t - (total - 0.38)) / 0.38),
        crate::model::Design::Retro => 1.,
    };
    let glitch_from = total - VHS_CUT;
    if design == crate::model::Design::Retro && t >= glitch_from + VHS_CUT * 0.45 {
        // Second half of the glitch: the boot picture is gone, the page shows under the noise.
        vhs_cut(out, (t - glitch_from) / VHS_CUT);
        return;
    }
    match design {
        crate::model::Design::Glass => {
            out.shape(full, 0., rgb(0x000000));
            // A soft blue glow behind the mark, swelling slowly.
            let swell = ease(t / 1.2);
            // Many faint layers make a smooth falloff without visible rings.
            for ring in 0..28 {
                let r = (220. + ring as f32 * 22.) * (0.85 + 0.15 * swell);
                out.shape(
                    Rect::new(780. - r, 540. - r * 0.8, 2. * r, 1.6 * r),
                    r * 0.8,
                    c(0x3c5cff, 0.011 * swell),
                );
            }
            let a = ease((t - 0.15) / 0.35);
            let grow = 0.92 + 0.08 * ease((t - 0.15) / 0.6);
            logo(out, assets, a, grow, false);
        }
        crate::model::Design::Retro => {
            out.shape(full, 0., rgb(0x000000));
            // CRT power-on: a bright line opens across the screen, then into the picture.
            let line = ease((t - 0.08) / 0.18);
            let open = ease((t - 0.28) / 0.2);
            if open < 1. && line > 0. {
                let w = 1920. * line;
                out.shape(
                    Rect::new(960. - w / 2., 538., w, 4.),
                    2.,
                    c(0xf2f6ff, 1. - open),
                );
            }
            if open > 0. {
                let h = 1080. * open;
                let picture = Rect::new(0., 540. - h / 2., 1920., h);
                const BANDS: usize = 12;
                for band in 0..BANDS {
                    let bh = picture.h / BANDS as f32;
                    out.shape(
                        Rect::new(0., picture.y + band as f32 * bh, 1920., bh + 0.5),
                        0.,
                        mix(PANEL_TOP, PANEL_BOTTOM, band as f32 / (BANDS - 1) as f32),
                    );
                }
                // A white flash as the picture comes up.
                out.shape(picture, 0., c(0xffffff, 0.5 * (1. - open)));
            }
            let a = ease((t - 0.5) / 0.25);
            logo(out, assets, a, 1., true);
            scanlines(out);
            if t >= glitch_from {
                vhs_cut(out, (t - glitch_from) / VHS_CUT);
            }
        }
    }
    out.fade(start, fade_out, 1.);
}

/// Length of the VHS glitch that ends the Retro TV boot, in seconds.
const VHS_CUT: f32 = 0.34;

/// A hard VHS tracking glitch over the whole screen (`p` runs 0 → 1): tear bars, dense noise
/// lines and a brightness flicker, strongest in the middle where the picture jumps.
fn vhs_cut(out: &mut Scene, p: f32) {
    let p = p.clamp(0., 1.);
    let strength = 1. - (2. * p - 1.).abs() * 0.7;
    let mut seed = 0x2545_f491_u32 ^ ((p * 24.) as u32).wrapping_mul(0x9e37_79b9);
    let mut random = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f32 / 1000.
    };
    // Flicker: alternately darker and brighter frames.
    let flash = if ((p * 24.) as u32) & 1 == 0 {
        0xffffff
    } else {
        0x000000
    };
    out.shape(
        Rect::new(0., 0., 1920., 1080.),
        0.,
        c(flash, 0.12 * strength),
    );
    // A few wide tear bars, like the picture losing sync.
    for _ in 0..3 {
        let y = random() * 1080.;
        let h = 20. + random() * 70.;
        out.shape(
            Rect::new(0., y, 1920., h),
            0.,
            c(0x9fb2ff, (0.18 + random() * 0.2) * strength),
        );
        out.shape(
            Rect::new(0., y + h, 1920., 4.),
            0.,
            c(0x000000, 0.6 * strength),
        );
    }
    // Dense broken noise lines across the screen.
    for _ in 0..34 {
        let y = random() * 1080.;
        let mut x = random() * 200. - 200.;
        while x < 1920. {
            let w = 30. + random() * 260.;
            out.shape(
                Rect::new(x, y, w, 3. + random() * 5.),
                0.,
                c(0xffffff, (0.2 + random() * 0.5) * strength),
            );
            x += w + random() * 160.;
        }
    }
}

/// The RobOS mark and wordmark, centred: soft Inter for Glass, outlined capitals for Retro.
fn logo(out: &mut Scene, assets: &Assets, alpha: f32, grow: f32, retro: bool) {
    if alpha <= 0. {
        return;
    }
    let mark = 170. * grow;
    let size = 120. * grow;
    let (rob, os) = if retro { ("ROB", "OS") } else { ("Rob", "OS") };
    let style = if retro {
        text(size, 800, 0.)
    } else {
        text(size, 500, -0.02)
    };
    let wide = assets.width(rob, style) + assets.width(os, style);
    let gap = 40. * grow;
    let x = 960. - (mark + gap + wide) / 2.;
    out.icon(
        assets,
        "robos-mark",
        Rect::new(x, 540. - mark / 2., mark, mark),
        c(0xffffff, alpha),
    );
    let tx = x + mark + gap;
    let ty = 540. - size * 0.62;
    let rob_w = assets.width(rob, style);
    if retro {
        let start = out.draw.len();
        outlined(
            out,
            rob,
            Rect::new(tx, ty, rob_w + 20., size * 1.3),
            size,
            WHITE,
            false,
        );
        outlined(
            out,
            os,
            Rect::new(tx + rob_w, ty, wide, size * 1.3),
            size,
            YELLOW,
            false,
        );
        out.fade(start, alpha, 1.);
    } else {
        out.text(
            rob,
            Rect::new(tx, ty, rob_w + 20., size * 1.3),
            style,
            c(0xf5f5f5, alpha),
            false,
        );
        out.text(
            os,
            Rect::new(tx + rob_w, ty, wide, size * 1.3),
            style,
            c(YELLOW, alpha),
            false,
        );
    }
}
