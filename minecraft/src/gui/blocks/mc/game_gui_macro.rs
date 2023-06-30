

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
    /*{
        /*
        let bg_image_size = Extent2::new(
            $bg_image_size_x as f32,
            $bg_image_size_y as f32,
        );

        logical_size(bg_image_size * 2.0,
            layer((
                $bg_image,
                layer(($(
                    {
                        let frac = Extent2::new($x as f32, $y as f32) / bg_image_size;
                        align_start(frac, $elem)
                    },
                )*))
            ))
        )*/
    }*/
    };
}


#[macro_export]
macro_rules! item_grid {
    ($cols:expr, $rows:expr, $array:expr $(,)?)=>{{
        let mut row_nums = [0; $rows];
        for (i, n) in row_nums.iter_mut().enumerate() {
            *n = i;
        }
        let array = $array;
        logical_width(0.0, // lol
            v_stack(0.0,
                row_nums.map(|n|
                    h_align(0.0,
                        logical_height(DEFAULT_SLOT_SIZE,
                            h_stack(0.0,
                                array_each(
                                    array_const_slice::<_, $cols>(array, n * $cols)
                                ).map(|slot|
                                    logical_width(DEFAULT_SLOT_SIZE,
                                        slot.gui(Default::default())
                                    )
                                )
                            )
                        )
                    )
                )
            )
        )
    }};
}


pub use game_gui;
pub use item_grid;
