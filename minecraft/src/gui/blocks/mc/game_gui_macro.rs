
#[macro_export]
macro_rules! game_gui {
    (
        bg_image: $bg_image:expr,
        bg_image_size: [$bg_image_size_x:expr, $bg_image_size_y:expr],
        $( [$x:expr, $y:expr] => $elem:expr ),*$(,)?
    )=>{
        relative(
            (),
            logical_size(
                Extent2::new(
                    $bg_image_size_x as f32,
                    $bg_image_size_y as f32,
                ),
                $bg_image,
            ),
            gui_seq_flatten!(
                
            ),
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

pub use game_gui;
