

#[macro_export]
macro_rules! game_gui {
    (
        [$bg_w:expr, $bg_h:expr],
        $bg_image:expr,
        [$(
            ([$x:expr, $y:expr], $elem:expr)
        ),*$(,)?]
    )=>{
        relative(
            (),
            logical_size([$bg_w as f32 * 2.0, $bg_h as f32 * 2.0],
                $bg_image,
            ),
            ($(
                logical_translate([$x as f32 * 2.0, $y as f32 * 2.0],
                    $elem
                ),
            )*),
        )
    };
}

pub use game_gui;
