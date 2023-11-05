
use super::GameDataBuilder;


macro_rules! content {
    ($($name:ident,)*)=>{
        
        $(
            pub mod $name;
        )*

        /// All content modules, initialized.
        #[derive(Debug)]
        pub struct ContentModules {$(
            pub $name: $name::ContentModule,
        )*}

        impl ContentModules {
            /// Initialize all content modules.
            pub fn init(builder: &mut GameDataBuilder) -> Self {
                ContentModules {$(
                    $name: $name::ContentModule::init(builder),
                )*}
            }
        }
    };
}

content!(
    air,
    stone,
    chest,
);
