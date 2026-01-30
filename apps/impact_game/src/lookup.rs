//! Looking up game state from Roc script.

macro_rules! define_lookup_target_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $variant,
            )*
        }

        #[::roc_integration::roc(imports=[Lookup])]
        impl $name {
            pub fn lookup_and_write_roc_bytes(
                &self,
                game: &$crate::Game,
                buffer: &mut [u8],
            ) -> ::anyhow::Result<()> {
                use ::roc_integration::Roc as _;
                match self {
                    $(
                        Self::$variant => $variant::lookup(game).write_roc_bytes(buffer),
                    )*
                }
            }

            pub fn roc_serialized_size(&self) -> usize {
                use ::roc_integration::Roc as _;
                match self {
                    $(
                        Self::$variant => $variant::SERIALIZED_SIZE,
                    )*
                }
            }

            #[::roc_integration::roc(
                name = "lookup",
                body = "[] |> write_bytes(self) |> Lookup.lookup!(decode)",
                effectful = true,
                ignore_types = true
            )]
            #[allow(unused_variables)]
            fn __lookup_for_roc(&self, decode: ()) {
                unimplemented!()
            }
        }
    };
}

#[macro_export]
macro_rules! define_lookup_type {
    (
        $(#[$meta:meta])*
        $vis:vis struct $target:ident $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis struct $target $($rest)*

        const _: $crate::lookup::GameLookupTarget = $crate::lookup::GameLookupTarget::$target;

        #[::roc_integration::roc(dependencies=[$crate::lookup::GameLookupTarget])]
        impl $target {
            /// Fetch the current value of this quantity.
            #[::roc_integration::roc(
                name = "get",
                body = ["Lookup.GameLookupTarget.lookup!(", $target, ", from_bytes)"],
                effectful = true
            )]
            fn __lookup_for_roc() -> ::anyhow::Result<Self> {
                unimplemented!()
            }
        }
    };
}

use crate::player::inventory::InventoryMass;
use roc_integration::roc;

define_lookup_target_enum! {
    #[roc(parents = "Lookup")]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum GameLookupTarget {
        InventoryMass,
    }
}
