// Copyright The Glide Authors
// SPDX-License-Identifier: MIT OR Apache-2.0

use objc2_core_foundation::{CGPoint, CGRect};

use super::size::{Direction, Orientation};

/// Finds the screen next to the screen `from` in `direction`.
///
/// `screens` pairs each screen's key with its frame. macOS arranges screens so
/// they touch without overlapping. The adjacent screen is one whose edge touches
/// the current screen's edge in `direction` and that overlaps it on the other
/// axis. If several do, the one whose center is closest on the other axis wins.
///
/// If there is none, this wraps around to the farthest screen in the opposite
/// direction that overlaps the current one on the other axis.
pub fn adjacent_screen<K: Copy + PartialEq>(
    screens: &[(K, CGRect)],
    from: K,
    direction: Direction,
) -> Option<K> {
    let &(_, cur) = screens.iter().find(|&&(k, _)| k == from)?;
    let horizontal = direction.orientation() == Orientation::Horizontal;
    let axis = |p: CGPoint| if horizontal { p.x } else { p.y };
    let cross = |p: CGPoint| if horizontal { p.y } else { p.x };

    let in_line: Vec<_> = screens
        .iter()
        .filter(|&&(k, frame)| {
            k != from
                && cross(frame.min()) < cross(cur.max())
                && cross(frame.max()) > cross(cur.min())
        })
        .map(|&(k, frame)| {
            let gap = match direction {
                Direction::Right | Direction::Down => axis(frame.min()) - axis(cur.max()),
                Direction::Left | Direction::Up => axis(cur.min()) - axis(frame.max()),
            };
            let cross_distance = (cross(frame.mid()) - cross(cur.mid())).abs();
            (k, gap, cross_distance)
        })
        .collect();

    let touching = in_line.iter().filter(|s| s.1 == 0.0).min_by(|a, b| a.2.total_cmp(&b.2));
    let wrapped = || in_line.iter().min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));
    touching.or_else(wrapped).map(|&(k, ..)| k)
}

#[cfg(test)]
mod tests {
    use objc2_core_foundation::CGSize;

    use super::*;
    use crate::model::Direction::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> CGRect {
        CGRect::new(
            CGPoint::new(f64::from(x), f64::from(y)),
            CGSize::new(f64::from(w), f64::from(h)),
        )
    }

    fn keyed(frames: &[CGRect]) -> Vec<(usize, CGRect)> {
        frames.iter().copied().enumerate().collect()
    }

    #[test]
    fn side_by_side() {
        // +--------+--------+--------+
        // |   0    |   2    |   1    |
        // +--------+--------+--------+
        let frames = [
            rect(0, 0, 1000, 800),
            rect(2000, 0, 1000, 800),
            rect(1000, 0, 1000, 800),
        ];
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Right), Some(2));
        assert_eq!(adjacent_screen(&keyed(&frames), 2, Right), Some(1));
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Left), Some(2));
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Right), Some(0));
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Left), Some(1));
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Up), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Down), None);
    }

    #[test]
    fn stacked_with_offsets() {
        //   +----------------+
        //   |       0        |
        // +-+-------+--------+
        // |    1    |
        // ++-----+--+
        //  |  2  |
        //  +-----+
        let frames = [
            rect(0, 0, 2560, 1440),
            rect(-300, 1440, 1512, 982),
            rect(-200, 2422, 1000, 800),
        ];
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Down), Some(1));
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Down), Some(2));
        assert_eq!(adjacent_screen(&keyed(&frames), 2, Up), Some(1));
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Up), Some(0));
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Up), Some(2));
        assert_eq!(adjacent_screen(&keyed(&frames), 2, Down), Some(0));
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Left), None);
    }

    #[test]
    fn mixed() {
        // +-----------+-----------+
        // |     0     |     1     |
        // +-----------+-+--------++
        //               |   2    |
        //               +--------+
        let frames = [
            rect(0, 0, 1920, 1080),
            rect(1920, 0, 1920, 1080),
            rect(2200, 1080, 1512, 982),
        ];
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Down), Some(2));
        assert_eq!(adjacent_screen(&keyed(&frames), 2, Up), Some(1));
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Right), Some(1));
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Down), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 2, Left), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Right), Some(0));
        assert_eq!(adjacent_screen(&keyed(&frames), 2, Down), Some(1));
    }

    #[test]
    fn ignores_screens_touching_at_a_corner() {
        // +--------+
        // |   0    |
        // +--------+--------+
        //          |   1    |
        //          +--------+
        let frames = [rect(0, 0, 1000, 800), rect(1000, 800, 1000, 800)];
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Down), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Right), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Up), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 1, Left), None);
    }

    #[test]
    fn breaks_ties_by_cross_distance() {
        // +-------------------+
        // |         0         |
        // +-------+-+---------+
        // |   1   | |    2    |
        // +-------+ +---------+
        let frames = [
            rect(0, 0, 2000, 1000),
            rect(0, 1000, 800, 600),
            rect(1000, 1000, 1000, 600),
        ];
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Down), Some(2));
    }

    #[test]
    fn ignores_screens_overlapping_along_the_direction() {
        // +-----------+
        // |           +-----------+
        // |     0     |     1     |
        // +-----------+           |
        //             +-----------+
        let frames = [rect(0, 0, 1920, 1080), rect(1920, 20, 1920, 1080)];
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Down), None);
        assert_eq!(adjacent_screen(&keyed(&frames), 0, Right), Some(1));
    }
}
