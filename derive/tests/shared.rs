// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! The shared derives must behave exactly like the standard ones, so each
//! shape is declared twice and every observable result is compared.

use core::hash::{Hash, Hasher};
use std::hash::DefaultHasher;

macro_rules! shapes {
    ($($derive:path),*) => {
        #[derive($($derive),*)]
        pub struct Unit;

        #[derive($($derive),*)]
        pub struct Empty {}

        #[derive($($derive),*)]
        pub struct Tuple(pub u8, pub String);

        #[derive($($derive),*)]
        pub struct Named {
            pub r#type: u8,
            pub name: String,
            pub children: Vec<Tree<u8>>,
        }

        #[derive($($derive),*)]
        pub enum Single {
            Only(u8),
        }

        #[derive($($derive),*)]
        pub enum Tree<T> {
            Leaf,
            Value(T),
            Pair(T, T),
            Node { r#type: T, children: Vec<Tree<T>> },
        }

        #[derive($($derive),*)]
        pub enum Mixed {
            A(Unit, Empty),
            B(Tuple),
            C { named: Named, single: Single },
            D,
        }

        pub fn samples() -> Vec<Mixed> {
            let named = |t: u8, name: &str, children: Vec<Tree<u8>>| Named {
                r#type: t,
                name: name.into(),
                children,
            };
            vec![
                Mixed::D,
                Mixed::A(Unit, Empty {}),
                Mixed::B(Tuple(1, "a".into())),
                Mixed::B(Tuple(1, "b".into())),
                Mixed::B(Tuple(2, "a".into())),
                Mixed::C { named: named(0, "x", vec![]), single: Single::Only(0) },
                Mixed::C { named: named(0, "x", vec![Tree::Leaf]), single: Single::Only(0) },
                Mixed::C { named: named(0, "x", vec![Tree::Value(3)]), single: Single::Only(0) },
                Mixed::C { named: named(0, "x", vec![Tree::Pair(3, 1)]), single: Single::Only(0) },
                Mixed::C { named: named(0, "x", vec![Tree::Pair(3, 2)]), single: Single::Only(1) },
                Mixed::C {
                    named: named(9, "y", vec![Tree::Node { r#type: 1, children: vec![Tree::Leaf] }]),
                    single: Single::Only(1),
                },
            ]
        }
    };
}

mod standard {
    shapes!(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash);
}

mod shared {
    use sqlparser_derive::{
        SharedClone, SharedDebug, SharedHash, SharedOrd, SharedPartialEq, SharedPartialOrd,
    };
    shapes!(
        SharedDebug,
        SharedClone,
        SharedPartialEq,
        Eq,
        SharedPartialOrd,
        SharedOrd,
        SharedHash
    );
}

fn digest(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn shared_derives_match_standard_derives() {
    let standard = standard::samples();
    let shared = shared::samples();
    for (s, o) in standard.iter().zip(&shared) {
        assert_eq!(format!("{s:?}"), format!("{o:?}"));
        assert_eq!(format!("{s:#?}"), format!("{o:#?}"));
        assert_eq!(format!("{:?}", s.clone()), format!("{:?}", o.clone()));
        assert_eq!(digest(s), digest(o), "{s:?}");
    }
    for (i, (s1, o1)) in standard.iter().zip(&shared).enumerate() {
        for (j, (s2, o2)) in standard.iter().zip(&shared).enumerate() {
            assert_eq!(s1 == s2, o1 == o2, "eq {i} {j}");
            assert_eq!(
                s1.partial_cmp(s2),
                o1.partial_cmp(o2),
                "partial_cmp {i} {j}"
            );
            assert_eq!(s1.cmp(s2), o1.cmp(o2), "cmp {i} {j}");
        }
    }
}
