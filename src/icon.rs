//! The tray icon: the MeshLLM logo in colour with a green badge while the
//! node answers, and in grey while it does not.

use crate::node::NodeState;
use tray_icon::Icon;

const SIZE: u32 = 32;
/// The 32x32 favicon of the MeshLLM console (mesh-llm-ui/public), as raw RGBA.
const LOGO: &[u8] = include_bytes!("../assets/meshllm-32.rgba");
const BADGE_GREEN: [u8; 3] = [0x2e, 0xa0, 0x5c];

pub fn for_state(state: NodeState) -> Icon {
    let rgba = match state {
        NodeState::Running => with_badge(LOGO),
        NodeState::Stopped => greyed(LOGO),
    };
    Icon::from_rgba(rgba, SIZE, SIZE).expect("a 32x32 RGBA buffer is a valid icon")
}

/// The logo with a green dot, ringed in white, in the bottom-right corner.
fn with_badge(logo: &[u8]) -> Vec<u8> {
    let mut rgba = logo.to_vec();
    let (cx, cy) = (24.5_f32, 24.5_f32);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let color = if d <= 5.0 {
                BADGE_GREEN
            } else if d <= 6.5 {
                [0xff, 0xff, 0xff]
            } else {
                continue;
            };
            let i = ((y * SIZE + x) * 4) as usize;
            rgba[i..i + 3].copy_from_slice(&color);
            rgba[i + 3] = 0xff;
        }
    }
    rgba
}

/// The logo in grey, a little faded, the usual look of something switched off.
fn greyed(logo: &[u8]) -> Vec<u8> {
    logo.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|px| {
            let luma = (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) as u8;
            [luma, luma, luma, (px[3] as f32 * 0.75) as u8]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_is_a_32_by_32_rgba_image() {
        assert_eq!(LOGO.len(), (SIZE * SIZE * 4) as usize);
    }

    #[test]
    fn running_adds_a_green_badge_and_stopped_drops_the_colour() {
        let badge = with_badge(LOGO);
        let center = ((24 * SIZE + 24) * 4) as usize;
        assert_eq!(&badge[center..center + 4], &[0x2e, 0xa0, 0x5c, 0xff]);
        let grey = greyed(LOGO);
        assert!(
            grey.as_chunks::<4>()
                .0
                .iter()
                .all(|px| px[0] == px[1] && px[1] == px[2])
        );
    }
}
