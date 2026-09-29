// Copyright The Glide Authors
// SPDX-License-Identifier: MIT OR Apache-2.0

use objc2_core_foundation::CGRect;

use super::size::{Direction, Orientation};

/// How far screen edges can be apart and still count as touching.
const EDGE_TOLERANCE: f64 = 1.0;

/// Finds the screen next to `frames[from]` in `direction`, returning its index.
///
/// Candidates are screens lying entirely past the current screen's edge in
/// that direction. Screens that overlap the current one on the other axis are
/// preferred, then the closest along the direction, then the closest on the
/// other axis. There is no wraparound.
pub fn adjacent_screen(frames: &[CGRect], from: usize, direction: Direction) -> Option<usize> {
    let cur = *frames.get(from)?;
    let horizontal = direction.orientation() == Orientation::Horizontal;
    let (cur_min, cur_max) = span(cur, horizontal);
    let (cur_cross_min, cur_cross_max) = span(cur, !horizontal);
    let cur_cross_mid = (cur_cross_min + cur_cross_max) / 2.0;

    frames
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != from)
        .filter_map(|(i, &frame)| {
            let (min, max) = span(frame, horizontal);
            let gap = match direction {
                Direction::Right | Direction::Down => min - cur_max,
                Direction::Left | Direction::Up => cur_min - max,
            };
            if gap < -EDGE_TOLERANCE {
                return None;
            }
            let (cross_min, cross_max) = span(frame, !horizontal);
            let overlaps = cross_min < cur_cross_max && cross_max > cur_cross_min;
            let cross_distance = ((cross_min + cross_max) / 2.0 - cur_cross_mid).abs();
            Some((i, !overlaps, gap.max(0.0), cross_distance))
        })
        .min_by(|a, b| a.1.cmp(&b.1).then(a.2.total_cmp(&b.2)).then(a.3.total_cmp(&b.3)))
        .map(|(i, ..)| i)
}

/// The extent of `rect` along the x axis if `horizontal`, otherwise the y axis.
fn span(rect: CGRect, horizontal: bool) -> (f64, f64) {
    if horizontal {
        (rect.origin.x, rect.origin.x + rect.size.width)
    } else {
        (rect.origin.y, rect.origin.y + rect.size.height)
    }
}

#[cfg(test)]
mod tests {
    use objc2_core_foundation::{CGPoint, CGSize};

    use super::*;
    use crate::model::Direction::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> CGRect {
        CGRect::new(
            CGPoint::new(f64::from(x), f64::from(y)),
            CGSize::new(f64::from(w), f64::from(h)),
        )
    }

    #[test]
    fn side_by_side() {
        let frames = [
            rect(0, 0, 1000, 800),
            rect(2000, 0, 1000, 800),
            rect(1000, 0, 1000, 800),
        ];
        assert_eq!(adjacent_screen(&frames, 0, Right), Some(2));
        assert_eq!(adjacent_screen(&frames, 2, Right), Some(1));
        assert_eq!(adjacent_screen(&frames, 1, Left), Some(2));
        assert_eq!(adjacent_screen(&frames, 1, Right), None);
        assert_eq!(adjacent_screen(&frames, 0, Left), None);
        assert_eq!(adjacent_screen(&frames, 0, Up), None);
        assert_eq!(adjacent_screen(&frames, 0, Down), None);
    }

    #[test]
    fn stacked_with_offsets() {
        // A wide screen with two smaller screens below it, the lower ones
        // starting further left.
        let frames = [
            rect(0, 0, 2560, 1440),
            rect(-300, 1440, 1512, 982),
            rect(-200, 2422, 1000, 800),
        ];
        assert_eq!(adjacent_screen(&frames, 0, Down), Some(1));
        assert_eq!(adjacent_screen(&frames, 1, Down), Some(2));
        assert_eq!(adjacent_screen(&frames, 2, Up), Some(1));
        assert_eq!(adjacent_screen(&frames, 1, Up), Some(0));
        assert_eq!(adjacent_screen(&frames, 0, Up), None);
        assert_eq!(adjacent_screen(&frames, 0, Left), None);
    }

    #[test]
    fn mixed() {
        // Two monitors side by side with a laptop below the right one.
        let frames = [
            rect(0, 0, 1920, 1080),
            rect(1920, 0, 1920, 1080),
            rect(2200, 1080, 1512, 982),
        ];
        assert_eq!(adjacent_screen(&frames, 1, Down), Some(2));
        assert_eq!(adjacent_screen(&frames, 2, Up), Some(1));
        assert_eq!(adjacent_screen(&frames, 0, Right), Some(1));
        assert_eq!(adjacent_screen(&frames, 0, Down), Some(2));
        assert_eq!(adjacent_screen(&frames, 2, Left), Some(0));
    }
}
