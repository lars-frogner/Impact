//! Looking up game state from Roc script.

macro_rules! define_lookup_target_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $variant:ident $( { $( $argn:ident : $argt:ty ),* $(,)? } )?
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $variant $( { $( $argn : $argt ),* } )?,
            )*
        }

        #[::roc_integration::roc(imports=[Lookup])]
        impl $name {
            pub fn lookup_and_write_roc_bytes(
                self,
                game: &$crate::Game,
                buffer: &mut [u8],
            ) -> ::anyhow::Result<()> {
                use ::roc_integration::Roc as _;
                match self {
                    $(
                        Self::$variant $( { $( $argn ),* } )? => {
                            $variant::lookup(game $( ,$( $argn ),* )? )?.write_roc_bytes(buffer)
                        }
                    )*
                }
            }

            pub fn roc_serialized_size(&self) -> usize {
                use ::roc_integration::Roc as _;
                match self {
                    $(
                        Self::$variant $( { $( $argn: _ ),* } )? => $variant::SERIALIZED_SIZE,
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
        variant = $variant:ident $( { $( $argn:ident : $argt:ty ),* $(,)? } )?;
        $(#[$meta:meta])*
        $vis:vis struct $name:ident $($rest:tt)*
    ) => {
        $(#[$meta])*
        $vis struct $name $($rest)*

        // Statically assert that the enum variant exists
        const _: fn($( $( $argt ),* )?) -> $crate::lookup::GameLookupTarget =
            |$( $( $argn : $argt ),* )?| {
                $crate::lookup::GameLookupTarget::$variant $( { $( $argn ),* } )?
            };

        #[::roc_integration::roc(dependencies=[$crate::lookup::GameLookupTarget $( $( ,$argt )* )? ])]
        impl $name {
            /// Fetch the current value of this quantity.
            #[::roc_integration::roc(
                name = "get",
                body = ["Lookup.GameLookupTarget.lookup!(", $variant, $( "{" ,$( $argn ,", " ),* ,"}", )? ", from_bytes)"],
                effectful = true
            )]
            #[allow(unused_variables)]
            fn __lookup_for_roc( $( $( $argn : $argt ),* )? ) -> ::anyhow::Result<Self> {
                unimplemented!()
            }
        }
    };
}

use crate::player::{
    inventory::InventoryMass,
    tools::{CapsuleAbsorbedVoxelMass, SphereAbsorbedVoxelMass},
};
use impact::impact_ecs::world::EntityID;
use roc_integration::roc;

define_lookup_target_enum! {
    #[roc(parents = "Lookup")]
    #[derive(Clone, Debug)]
    pub enum GameLookupTarget {
        InventoryMass,
        SphereAbsorbedVoxelMass { entity_id: EntityID },
        CapsuleAbsorbedVoxelMass { entity_id: EntityID },
    }
}
