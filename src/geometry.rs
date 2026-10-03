/// Restore at the current position, keeping the title bar in the work area.
pub fn restore_position(
    x: i32,
    y: i32,
    width: i32,
    caption_height: i32,
    work: [i32; 4],
) -> (i32, i32) {
    (
        x.clamp(work[0], (work[2] - width).max(work[0])),
        y.clamp(work[1], (work[3] - caption_height).max(work[1])),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_window_unrolls_at_new_position_but_stays_reachable() {
        assert_eq!(
            restore_position(100, 200, 800, 32, [0, 0, 1920, 1040]),
            (100, 200)
        );
        assert_eq!(
            restore_position(1900, 1100, 800, 32, [0, 0, 1920, 1040]),
            (1120, 1008)
        );
        assert_eq!(
            restore_position(-1800, 50, 800, 32, [-1920, 0, 0, 1040]),
            (-1800, 50)
        );
        assert_eq!(
            restore_position(-3000, -400, 2200, 32, [0, 0, 1920, 1040]),
            (0, 0)
        );
    }

    #[test]
    fn tall_caption_stays_visible_at_bottom_of_work_area() {
        assert_eq!(
            restore_position(100, 1030, 800, 60, [0, 0, 1920, 1040]),
            (100, 980)
        );
    }
}
