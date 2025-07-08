// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

macro_rules! _default_val {
    (@verbatim: $s:literal, $ty:ty) => {{
        let default_: $ty = serde_json::from_str(&$s).unwrap();
        default_
    }};
    ($default:expr, $ty:ty) => {{
        let default_: $ty = $default;
        default_
    }};
}
pub(crate) use _default_val as default_val;

macro_rules! _default_str {
    (@verbatim: $s:literal, $_ty:ty) => {
        $s.to_owned()
    };
    ($default:expr, $ty:ty) => {{
        let val = default_val!($default, $ty);
        serde_json::to_string_pretty(&val).unwrap()
    }};
}
pub(crate) use _default_str as default_str;

macro_rules! _impl_for_config_data {
    (local, $(
            $(#[doc=$doc:literal])*
            $vis:vis $field:ident : $ty:ty = $default:expr,
        )*
    ) => {
        impl crate::Config {
            $(
                $($doc)*
                #[allow(non_snake_case)]
                pub(crate) fn $field(&self, source_root: Option<PackageId>) -> &$ty {
                    let mut source_root = source_root.as_ref();
                    while let Some(sr) = source_root {
                        if let Some((file, _)) = self.ratoml_file.get(&sr) {
                            match file {
                                RatomlFile::Workspace(config) => {
                                    if let Some(v) = config.local.$field.as_ref() {
                                        return &v;
                                    }
                                },
                                RatomlFile::Crate(config) => {
                                    if let Some(value) = config.$field.as_ref() {
                                        return value;
                                    }
                                }
                            }
                        }
                        source_root = self.source_root_parent_map.get(&sr);
                    }

                    if let Some(v) = self.client_config.0.local.$field.as_ref() {
                        return &v;
                    }

                    if let Some((user_config, _)) = self.user_config.as_ref() {
                        if let Some(v) = user_config.local.$field.as_ref() {
                            return &v;
                        }
                    }

                    &self.default_config.local.$field
                }
            )*
        }
    };
    (global, $(
            $(#[doc=$doc:literal])*
            $vis:vis $field:ident : $ty:ty = $default:expr,
        )*
    ) => {
        impl crate::Config {
            $(
                $($doc)*
                #[allow(non_snake_case)]
                pub(crate) fn $field(&self) -> &$ty {
                    if let Some(v) = self.client_config.0.global.$field.as_ref() {
                        return &v;
                    }

                    // if let Some((user_config, _)) = self.user_config.as_ref() {
                    //     if let Some(v) = user_config.global.$field.as_ref() {
                    //         return &v;
                    //     }
                    // }

                    &self.default_config.global.$field
                }
            )*
        }
    };
    (client, $(
            $(#[doc=$doc:literal])*
            $vis:vis $field:ident : $ty:ty = $default:expr,
       )*
    ) => {
        impl crate::Config {
            $(
                $($doc)*
                #[allow(non_snake_case)]
                pub(crate) fn $field(&self) -> &$ty {
                    if let Some(v) = self.client_config.0.client.$field.as_ref() {
                        return &v;
                    }

                    &self.default_config.client.$field
                }
            )*
        }
    };
}
pub(crate) use _impl_for_config_data as impl_for_config_data;

macro_rules! _config_data {
    // modname is for the tests
    ($(#[doc=$dox:literal])* $modname:ident: struct $name:ident <- $input:ident -> {
        $(
            $(#[doc=$doc:literal])*
            $vis:vis $field:ident $(| $alias:ident)*: $ty:ty = $(@$marker:ident: )? $default:expr,
        )*
    }) => {
        /// Default config values for this grouping.
        #[allow(non_snake_case)]
        #[derive(Debug, Clone, PartialEq)]
        struct $name { $($field: $ty,)* }

        impl_for_config_data!{
            $modname,
            $(
                $vis $field : $ty = $default,
            )*
        }

        /// All fields `Option<T>`, `None` representing fields not set in a particular JSON/TOML blob.
        #[allow(non_snake_case)]
        #[derive(Clone, Default, PartialEq)]
        struct $input { $(
            $field: Option<$ty>,
        )* }

        impl std::fmt::Debug for $input {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut s = f.debug_struct(stringify!($input));
                $(
                    if let Some(val) = self.$field.as_ref() {
                        s.field(stringify!($field), val);
                    }
                )*
                s.finish()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name {$(
                    $field: default_val!($(@$marker:)? $default, $ty),
                )*}
            }
        }

        #[allow(unused, clippy::ptr_arg)]
        impl $input {
            const FIELDS: &'static [&'static str] = &[$(stringify!($field)),*];

            fn from_json(json: &mut serde_json::Value, error_sink: &mut Vec<(String, serde_json::Error)>) -> Self {
                Self {$(
                    $field: get_field_json(
                        json,
                        error_sink,
                        stringify!($field),
                        None$(.or(Some(stringify!($alias))))*,
                    ),
                )*}
            }

            fn schema_fields(sink: &mut Vec<SchemaField>) {
                sink.extend_from_slice(&[
                    $({
                        let field = stringify!($field);
                        let ty = stringify!($ty);
                        let default = default_str!($(@$marker:)? $default, $ty);

                        (field, ty, &[$($doc),*], default)
                    },)*
                ])
            }
        }

        mod $modname {
            // #[test]
            // fn fields_are_sorted() {
            //     super::$input::FIELDS.windows(2).for_each(|w| assert!(w[0] <= w[1], "{} <= {} does not hold", w[0], w[1]));
            // }
        }
    };
}
pub(crate) use _config_data as config_data;
