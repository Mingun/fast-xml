use fast_xml::de::Deserializer;
use fast_xml::utils::ByteBuf;
use fast_xml::DeError;

use pretty_assertions::assert_eq;

use serde::de::IgnoredAny;
use serde::serde_if_integer128;
use serde::Deserialize;

/// Deserialize an instance of type T from a string of XML text.
/// If deserialization was succeeded checks that all XML events was consumed
fn from_str<'de, T>(s: &'de str) -> Result<T, DeError>
where
    T: Deserialize<'de>,
{
    // Log XML that we try to deserialize to see it in the failed tests output
    dbg!(s);
    let mut de = Deserializer::from_str(s);
    let result = T::deserialize(&mut de);

    // If type was deserialized, the whole XML document should be consumed
    if let Ok(_) = result {
        match <()>::deserialize(&mut de) {
            Err(DeError::UnexpectedEof) => (),
            e => panic!("Expected end `UnexpectedEof`, but got {:?}", e),
        }
    }

    result
}

#[test]
fn string_borrow() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct BorrowedText<'a> {
        #[serde(rename = "$value")]
        text: &'a str,
    }

    let borrowed_item: BorrowedText = from_str("<text>Hello world</text>").unwrap();

    assert_eq!(borrowed_item.text, "Hello world");
}

#[derive(Debug, Deserialize, PartialEq)]
struct Item {
    name: String,
    source: String,
}

#[test]
fn multiple_roots_attributes() {
    let item: Vec<Item> = from_str(
        r#"
            <item name="hello1" source="world1.rs" />
            <item name="hello2" source="world2.rs" />
        "#,
    )
    .unwrap();
    assert_eq!(
        item,
        vec![
            Item {
                name: "hello1".to_string(),
                source: "world1.rs".to_string(),
            },
            Item {
                name: "hello2".to_string(),
                source: "world2.rs".to_string(),
            },
        ]
    );
}

#[test]
fn nested_collection() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Project {
        name: String,

        #[serde(rename = "item", default)]
        items: Vec<Item>,
    }

    let project: Project = from_str(
        r#"
        <project name="my_project">
            <item name="hello1" source="world1.rs" />
            <item name="hello2" source="world2.rs" />
        </project>
        "#,
    )
    .unwrap();
    assert_eq!(
        project,
        Project {
            name: "my_project".to_string(),
            items: vec![
                Item {
                    name: "hello1".to_string(),
                    source: "world1.rs".to_string(),
                },
                Item {
                    name: "hello2".to_string(),
                    source: "world2.rs".to_string(),
                },
            ],
        }
    );
}

#[test]
fn collection_of_enums() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum MyEnum {
        A(String),
        B { name: String, flag: bool },
        C,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct MyEnums {
        // TODO: This should be #[serde(flatten)], but right now serde don't support flattening of sequences
        // See https://github.com/serde-rs/serde/issues/1905
        #[serde(rename = "$value")]
        items: Vec<MyEnum>,
    }

    let s = r#"
    <enums>
        <A>test</A>
        <B name="hello" flag="t" />
        <C />
    </enums>
    "#;

    let project: MyEnums = from_str(s).unwrap();

    assert_eq!(
        project,
        MyEnums {
            items: vec![
                MyEnum::A("test".to_string()),
                MyEnum::B {
                    name: "hello".to_string(),
                    flag: true,
                },
                MyEnum::C,
            ],
        }
    );
}

#[test]
fn deserialize_bytes() {
    let item: ByteBuf = from_str(r#"<item>bytes</item>"#).unwrap();

    assert_eq!(item, ByteBuf(b"bytes".to_vec()));
}

/// Test for https://github.com/tafia/quick-xml/issues/231
#[test]
fn implicit_value() {
    use serde_value::Value;

    let item: Value = from_str(r#"<root>content</root>"#).unwrap();

    assert_eq!(
        item,
        Value::Map(
            vec![(
                Value::String("$value".into()),
                Value::String("content".into())
            )]
            .into_iter()
            .collect()
        )
    );
}

#[test]
fn explicit_value() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Item {
        #[serde(rename = "$value")]
        content: String,
    }

    let item: Item = from_str(r#"<root>content</root>"#).unwrap();

    assert_eq!(
        item,
        Item {
            content: "content".into()
        }
    );
}

#[test]
fn without_value() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Item;

    let item: Item = from_str(r#"<root>content</root>"#).unwrap();

    assert_eq!(item, Item);
}

/// Tests calling `deserialize_ignored_any`
#[test]
fn ignored_any() {
    let err = from_str::<IgnoredAny>("");
    match err {
        Err(DeError::UnexpectedEof) => {}
        other => panic!("Expected `UnexpectedEof`, found {:?}", other),
    }

    from_str::<IgnoredAny>(r#"<empty/>"#).unwrap();
    from_str::<IgnoredAny>(r#"<with-attributes key="value"/>"#).unwrap();
    from_str::<IgnoredAny>(r#"<nested>text</nested>"#).unwrap();
    from_str::<IgnoredAny>(r#"<nested><![CDATA[cdata]]></nested>"#).unwrap();
    from_str::<IgnoredAny>(r#"<nested><nested/></nested>"#).unwrap();
}

/// Tests for trivial XML documents: empty or contains only primitive type
/// on a top level; all of them should be considered invalid
mod trivial {
    use super::*;

    #[rustfmt::skip] // excess spaces used for readability
    macro_rules! eof {
        ($name:ident: $type:ty = $value:expr) => {
            #[test]
            fn $name() {
                let item = from_str::<$type>($value).unwrap_err();

                match item {
                    DeError::UnexpectedEof => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", item),
                }
            }
        };
        ($value:expr) => {
            eof!(i8_:    i8    = $value);
            eof!(i16_:   i16   = $value);
            eof!(i32_:   i32   = $value);
            eof!(i64_:   i64   = $value);
            eof!(isize_: isize = $value);

            eof!(u8_:    u8    = $value);
            eof!(u16_:   u16   = $value);
            eof!(u32_:   u32   = $value);
            eof!(u64_:   u64   = $value);
            eof!(usize_: usize = $value);

            serde_if_integer128! {
                eof!(u128_: u128 = $value);
                eof!(i128_: i128 = $value);
            }

            eof!(f32_: f32 = $value);
            eof!(f64_: f64 = $value);

            eof!(false_: bool = $value);
            eof!(true_: bool = $value);
            eof!(char_: char = $value);

            eof!(string: String = $value);
            eof!(byte_buf: ByteBuf = $value);

            #[test]
            fn unit() {
                let item = from_str::<()>($value).unwrap_err();

                match item {
                    DeError::UnexpectedEof => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", item),
                }
            }
        };
    }

    /// Empty document should considered invalid no matter what type we try to deserialize
    mod empty_doc {
        use super::*;
        eof!("");
    }

    /// Document that contains only comment should be handled as if it is empty
    mod only_comment {
        use super::*;
        eof!("<!--comment-->");
    }

    /// Tests deserialization from top-level tag content: `<root>...content...</root>`
    mod struct_ {
        use super::*;

        /// Well-formed XML must have a single tag at the root level.
        /// Any XML tag can be modeled as a struct, and content of this tag are modeled as
        /// fields of this struct.
        ///
        /// Because we want to get access to unnamed content of the tag (usually, this internal
        /// XML node called `#text`) we use a rename to a special name `$value`
        #[derive(Debug, Deserialize, PartialEq)]
        struct Trivial<T> {
            #[serde(rename = "$value")]
            value: T,
        }

        macro_rules! in_struct {
            ($name:ident: $type:ty = $value:expr, $expected:expr) => {
                #[test]
                fn $name() {
                    let item: Trivial<$type> = from_str($value).unwrap();

                    assert_eq!(item, Trivial { value: $expected });

                    match from_str::<Trivial<$type>>(&format!("<outer>{}</outer>", $value)) {
                        // Expected unexpected start element `<root>`
                        Err(DeError::UnexpectedStart(tag)) => assert_eq!(tag, b"root"),
                        x => panic!(
                            r#"Expected `Err(DeError::UnexpectedStart("root"))`, but got `{:?}`"#,
                            x
                        ),
                    }
                }
            };
        }

        /// Tests deserialization from text content in a tag
        #[rustfmt::skip] // tests formatted in a table
        mod text {
            use super::*;
            use pretty_assertions::assert_eq;

            in_struct!(i8_:    i8    = "<root>-42</root>", -42i8);
            in_struct!(i16_:   i16   = "<root>-4200</root>", -4200i16);
            in_struct!(i32_:   i32   = "<root>-42000000</root>", -42000000i32);
            in_struct!(i64_:   i64   = "<root>-42000000000000</root>", -42000000000000i64);
            in_struct!(isize_: isize = "<root>-42000000000000</root>", -42000000000000isize);

            in_struct!(u8_:    u8    = "<root>42</root>", 42u8);
            in_struct!(u16_:   u16   = "<root>4200</root>", 4200u16);
            in_struct!(u32_:   u32   = "<root>42000000</root>", 42000000u32);
            in_struct!(u64_:   u64   = "<root>42000000000000</root>", 42000000000000u64);
            in_struct!(usize_: usize = "<root>42000000000000</root>", 42000000000000usize);

            serde_if_integer128! {
                in_struct!(u128_: u128 = "<root>420000000000000000000000000000</root>", 420000000000000000000000000000u128);
                in_struct!(i128_: i128 = "<root>-420000000000000000000000000000</root>", -420000000000000000000000000000i128);
            }

            in_struct!(f32_: f32 = "<root>4.2</root>", 4.2f32);
            in_struct!(f64_: f64 = "<root>4.2</root>", 4.2f64);

            in_struct!(false_: bool = "<root>false</root>", false);
            in_struct!(true_: bool = "<root>true</root>", true);
            in_struct!(char_: char = "<root>r</root>", 'r');

            in_struct!(string:   String  = "<root>escaped&#x20;string</root>", "escaped string".into());
            // Byte buffers gives access to the raw data from the input, so never treated as escaped
            // TODO: It is a bit unusual and it would be better completely forbid deserialization
            // into bytes, because XML cannot store any bytes natively. User should use some sort
            // of encoding to a string, for example, hex or base64
            in_struct!(byte_buf: ByteBuf = "<root>escaped&#x20;byte_buf</root>", ByteBuf(r"escaped&#x20;byte_buf".into()));
        }

        /// Tests deserialization from CDATA content in a tag.
        /// CDATA handling similar to text handling except that strings does not unescapes
        #[rustfmt::skip] // tests formatted in a table
        mod cdata {
            use super::*;
            use pretty_assertions::assert_eq;

            in_struct!(i8_:    i8    = "<root><![CDATA[-42]]></root>", -42i8);
            in_struct!(i16_:   i16   = "<root><![CDATA[-4200]]></root>", -4200i16);
            in_struct!(i32_:   i32   = "<root><![CDATA[-42000000]]></root>", -42000000i32);
            in_struct!(i64_:   i64   = "<root><![CDATA[-42000000000000]]></root>", -42000000000000i64);
            in_struct!(isize_: isize = "<root><![CDATA[-42000000000000]]></root>", -42000000000000isize);

            in_struct!(u8_:    u8    = "<root><![CDATA[42]]></root>", 42u8);
            in_struct!(u16_:   u16   = "<root><![CDATA[4200]]></root>", 4200u16);
            in_struct!(u32_:   u32   = "<root><![CDATA[42000000]]></root>", 42000000u32);
            in_struct!(u64_:   u64   = "<root><![CDATA[42000000000000]]></root>", 42000000000000u64);
            in_struct!(usize_: usize = "<root><![CDATA[42000000000000]]></root>", 42000000000000usize);

            serde_if_integer128! {
                in_struct!(u128_: u128 = "<root><![CDATA[420000000000000000000000000000]]></root>", 420000000000000000000000000000u128);
                in_struct!(i128_: i128 = "<root><![CDATA[-420000000000000000000000000000]]></root>", -420000000000000000000000000000i128);
            }

            in_struct!(f32_: f32 = "<root><![CDATA[4.2]]></root>", 4.2f32);
            in_struct!(f64_: f64 = "<root><![CDATA[4.2]]></root>", 4.2f64);

            in_struct!(false_: bool = "<root><![CDATA[false]]></root>", false);
            in_struct!(true_: bool = "<root><![CDATA[true]]></root>", true);
            in_struct!(char_: char = "<root><![CDATA[r]]></root>", 'r');

            // Escape sequences does not processed inside CDATA section
            in_struct!(string:   String  = "<root><![CDATA[escaped&#x20;string]]></root>", "escaped&#x20;string".into());
            in_struct!(byte_buf: ByteBuf = "<root><![CDATA[escaped&#x20;byte_buf]]></root>", ByteBuf(r"escaped&#x20;byte_buf".into()));
        }
    }
}

mod unit {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Unit;

    #[test]
    fn simple() {
        let data: Unit = from_str("<root/>").unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_attribute() {
        let data: Unit = from_str(r#"<root excess="attribute"/>"#).unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_element() {
        let data: Unit = from_str(r#"<root><excess>element</excess></root>"#).unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_text() {
        let data: Unit = from_str(r#"<root>excess text</root>"#).unwrap();
        assert_eq!(data, Unit);
    }

    #[test]
    fn excess_cdata() {
        let data: Unit = from_str(r#"<root><![CDATA[excess CDATA]]></root>"#).unwrap();
        assert_eq!(data, Unit);
    }
}

mod newtype {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Newtype(bool);

    #[test]
    fn simple() {
        let data: Newtype = from_str("<root>true</root>").unwrap();
        assert_eq!(data, Newtype(true));
    }

    #[test]
    fn excess_attribute() {
        let data: Newtype = from_str(r#"<root excess="attribute">true</root>"#).unwrap();
        assert_eq!(data, Newtype(true));
    }
}

mod tuple {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn simple() {
        let data: (f32, String) = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            "<root>42</root><root>answer</root>",
        )
        .unwrap();
        assert_eq!(data, (42.0, "answer".into()));
    }

    #[test]
    fn excess_attribute() {
        let data: (f32, String) = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root excess="attribute">42</root><root>answer</root>"#,
        )
        .unwrap();
        assert_eq!(data, (42.0, "answer".into()));
    }
}

mod tuple_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Tuple(f32, String);

    #[test]
    fn simple() {
        let data: Tuple = from_str("<root>42</root><root>answer</root>").unwrap();
        assert_eq!(data, Tuple(42.0, "answer".into()));
    }

    #[test]
    fn excess_attribute() {
        let data: Tuple = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root excess="attribute">42</root><root>answer</root>"#,
        )
        .unwrap();
        assert_eq!(data, Tuple(42.0, "answer".into()));
    }
}

mod seq {
    use super::*;

    /// Check that top-level sequences can be deserialized from the multi-root XML documents
    mod top_level {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn simple() {
            from_str::<[(); 3]>("<root/><root>42</root><root>answer</root>").unwrap();

            let data: Vec<()> = from_str("<root/><root>42</root><root>answer</root>").unwrap();
            assert_eq!(data, vec![(), (), ()]);
        }

        /// Special case: empty sequence
        #[test]
        fn empty() {
            from_str::<[(); 0]>("").unwrap();

            let data: Vec<()> = from_str("").unwrap();
            assert_eq!(data, vec![]);
        }

        /// Special case: one-element sequence
        #[test]
        fn one_element() {
            from_str::<[(); 1]>("<root/>").unwrap();
            from_str::<[(); 1]>("<root>42</root>").unwrap();
            from_str::<[(); 1]>("text").unwrap();
            from_str::<[(); 1]>("<![CDATA[cdata]]>").unwrap();

            let data: Vec<()> = from_str("<root/>").unwrap();
            assert_eq!(data, vec![()]);

            let data: Vec<()> = from_str("<root>42</root>").unwrap();
            assert_eq!(data, vec![()]);

            let data: Vec<()> = from_str("text").unwrap();
            assert_eq!(data, vec![()]);

            let data: Vec<()> = from_str("<![CDATA[cdata]]>").unwrap();
            assert_eq!(data, vec![()]);
        }

        #[test]
        fn excess_attribute() {
            from_str::<[(); 3]>(r#"<root/><root excess="attribute">42</root><root>answer</root>"#)
                .unwrap();

            let data: Vec<()> =
                from_str(r#"<root/><root excess="attribute">42</root><root>answer</root>"#)
                    .unwrap();
            assert_eq!(data, vec![(), (), ()]);
        }

        #[test]
        fn mixed_content() {
            from_str::<[(); 3]>(
                r#"
                <element/>
                text
                <![CDATA[cdata]]>
                "#,
            )
            .unwrap();

            let data: Vec<()> = from_str(
                r#"
                <element/>
                text
                <![CDATA[cdata]]>
                "#,
            )
            .unwrap();
            assert_eq!(data, vec![(), (), ()]);
        }
    }

    /// Tests where each sequence item have an identical name in an XML.
    /// That explicitly means that `enum`s as list elements are not supported
    /// in that case, because enum requires different tags.
    ///
    /// (by `enums` we mean [externally tagged enums] is serde terminology)
    ///
    /// [externally tagged enums]: https://serde.rs/enum-representations.html#externally-tagged
    mod fixed_name {
        use super::*;

        /// This module contains tests where size of the list have a compile-time size
        mod fixed_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: [(); 3],
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [(); 0],
                }

                from_str::<List>(r#"<root></root>"#).unwrap();
                from_str::<List>(r#"<root/>"#).unwrap();
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [(); 1],
                }

                from_str::<List>(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();
            }

            /// Fever elements than expected size of sequence, each element has
            /// the same name. Failure expected
            #[test]
            fn fever_elements() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => {
                        assert_eq!(e, "invalid length 2, expected an array of length 3")
                    }
                    e => panic!(
                        r#"Expected `Err(Custom("invalid length 2, expected an array of length 3"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// More elements than expected size of sequence, each element has
            /// the same name. Failure expected. If you wish to ignore excess
            /// elements, use the special type, that consume as much elements
            /// as possible, but ignores excess elements
            #[test]
            fn excess_elements() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("duplicate field `item`"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// Mixed content assumes, that some elements will have an internal
            /// name `$value`, so, unless field named the same, it is expected
            /// to fail
            #[test]
            fn mixed_content() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("missing field `item`"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// In those tests sequence should be deserialized from an XML
            /// with additional elements that is not defined in the struct.
            /// That fields should be skipped during deserialization
            mod unknown_items {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[test]
                fn before() {
                    from_str::<List>(
                        r#"
                        <root>
                            <unknown/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn after() {
                    from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <unknown/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <unknown/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from an XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    item: [(); 3],
                }

                #[test]
                fn before() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn after() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from an XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    item: [(); 3],
                    node: (),
                }

                #[test]
                fn before() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn after() {
                    from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;
                #[cfg(not(feature = "overlapped-lists"))]
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    item: [(); 3],
                    element: [(); 2],
                }

                #[test]
                fn splitted() {
                    from_str::<Pair>(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <item/>
                            <element/>
                            <item/>
                            <element/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    data.unwrap();

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: [usize; 3],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>41</item>
                        <item>42</item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap();
                assert_eq!(data, List { item: [41, 42, 43] });

                from_str::<List>(
                    r#"
                    <root>
                        <item>41</item>
                        <item><item>42</item></item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap_err();
            }
        }

        /// This module contains tests where size of the list have an unspecified size
        mod variable_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                item: Vec<()>,
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                        <item/>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![(), (), ()],
                    }
                );
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                let data: List = from_str(r#"<root></root>"#).unwrap();
                assert_eq!(data, List { item: vec![] });

                let data: List = from_str(r#"<root/>"#).unwrap();
                assert_eq!(data, List { item: vec![] });
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                let data: List = from_str(
                    r#"
                    <root>
                        <item/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(data, List { item: vec![()] });
            }

            /// Mixed content assumes, that some elements will have an internal
            /// name `$value`, so, unless field named the same, it is expected
            /// to fail
            #[test]
            fn mixed_content() {
                let data = from_str::<List>(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                );

                match data {
                    Err(DeError::Custom(e)) => assert_eq!(e, "missing field `item`"),
                    e => panic!(
                        r#"Expected `Err(Custom("missing field `item`"))`, but found {:?}"#,
                        e
                    ),
                }
            }

            /// In those tests sequence should be deserialized from the XML
            /// with additional elements that is not defined in the struct.
            /// That fields should be skipped during deserialization
            mod unknown_items {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn before() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <unknown/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: List = from_str(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <unknown/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        List {
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<List>(
                        r#"
                        <root>
                            <item/>
                            <unknown/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        List {
                            item: vec![(), (), ()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Default, Deserialize)]
                struct Root {
                    node: (),
                    item: Vec<()>,
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![(), (), ()],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: vec![(), (), ()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "duplicate field `item`")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Default, Deserialize)]
                struct Root {
                    item: Vec<()>,
                    node: (),
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![(), (), ()],
                            node: (),
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <item/>
                            <item/>
                            <item/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![(), (), ()],
                            node: (),
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <item/>
                            <node/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: vec![(), (), ()],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "duplicate field `item`")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Pair {
                    item: Vec<()>,
                    element: Vec<()>,
                }

                #[test]
                fn splitted() {
                    let data: Pair = from_str(
                        r#"
                        <root>
                            <element/>
                            <element/>
                            <item/>
                            <item/>
                            <item/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Pair {
                            item: vec![(), (), ()],
                            element: vec![(), ()],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Pair>(
                        r#"
                        <root>
                            <item/>
                            <element/>
                            <item/>
                            <element/>
                            <item/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Pair {
                            item: vec![(), (), ()],
                            element: vec![(), ()],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `item`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `item`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    item: Vec<usize>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <item>41</item>
                        <item>42</item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![41, 42, 43],
                    }
                );

                from_str::<List>(
                    r#"
                    <root>
                        <item>41</item>
                        <item><item>42</item></item>
                        <item>43</item>
                    </root>
                    "#,
                )
                .unwrap_err();
            }
        }
    }

    /// Check that sequences inside element can be deserialized.
    /// In terms of serde this is a sequence flatten into the struct:
    ///
    /// ```ignore
    /// struct Root {
    ///   #[serde(flatten)]
    ///   items: Vec<T>,
    /// }
    /// ```
    /// except that fact that this is not supported nowadays
    /// (https://github.com/serde-rs/serde/issues/1905)
    ///
    /// Because this is very frequently used pattern in the XML, quick-xml
    /// have a workaround for this. If a field will have a special name `$value`
    /// then any `xs:element`s in the `xs:sequence` / `xs:all`, except that
    /// which name matches the struct name, will be associated with this field:
    ///
    /// ```ignore
    /// struct Root {
    ///   field: U,
    ///   #[serde(rename = "$value")]
    ///   items: Vec<Enum>,
    /// }
    /// ```
    /// In this example `<field>` tag will be associated with a `field` field,
    /// but all other tags will be associated with an `items` field. Disadvantages
    /// of this approach that you can have only one field, but usually you don't
    /// want more
    mod variable_name {
        use super::*;
        use serde::de::{Deserializer, EnumAccess, VariantAccess, Visitor};
        use std::fmt::{self, Formatter};

        // NOTE: Derive could be possible once https://github.com/serde-rs/serde/issues/2126 is resolved
        macro_rules! impl_deserialize_choice {
            ($name:ident : $(($field:ident, $field_name:literal)),*) => {
                impl<'de> Deserialize<'de> for $name {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        #[derive(Deserialize)]
                        #[serde(field_identifier)]
                        #[serde(rename_all = "kebab-case")]
                        enum Tag {
                            $($field,)*
                            Other(String),
                        }

                        struct EnumVisitor;
                        impl<'de> Visitor<'de> for EnumVisitor {
                            type Value = $name;

                            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                                f.write_str("enum ")?;
                                f.write_str(stringify!($name))
                            }

                            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                            where
                                A: EnumAccess<'de>,
                            {
                                match data.variant()? {
                                    $(
                                        (Tag::$field, variant) => variant.unit_variant().map(|_| $name::$field),
                                    )*
                                    (Tag::Other(t), v) => v.unit_variant().map(|_| $name::Other(t)),
                                }
                            }
                        }

                        const VARIANTS: &'static [&'static str] = &[
                            $($field_name,)*
                            "<any other tag>"
                        ];
                        deserializer.deserialize_enum(stringify!($name), VARIANTS, EnumVisitor)
                    }
                }
            };
        }

        /// Type that can be deserialized from `<one>`, `<two>`, or any other element
        #[derive(Debug, PartialEq)]
        enum Choice {
            One,
            Two,
            /// Any other tag name except `One` or `Two`, name of tag stored inside variant
            Other(String),
        }
        impl_deserialize_choice!(Choice: (One, "one"), (Two, "two"));

        /// Type that can be deserialized from `<first>`, `<second>`, or any other element
        #[derive(Debug, PartialEq)]
        enum Choice2 {
            First,
            Second,
            /// Any other tag name except `First` or `Second`, name of tag stored inside variant
            Other(String),
        }
        impl_deserialize_choice!(Choice2: (First, "first"), (Second, "second"));

        /// Type that can be deserialized from `<one>`, `<two>`, or any other element.
        /// Used for `primitives` tests
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        enum Choice3 {
            One(usize),
            Two(String),
            #[serde(other)]
            Other,
        }

        /// This module contains tests where size of the list have a compile-time size
        mod fixed_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: [Choice; 3],
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice; 0],
                }

                from_str::<List>(r#"<root></root>"#).unwrap();
                from_str::<List>(r#"<root/>"#).unwrap();
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice; 1],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [Choice::One],
                    }
                );
            }

            /// Fever elements than expected size of sequence, each element has
            /// the same name. Failure expected
            #[test]
            fn fever_elements() {
                from_str::<List>(
                    r#"
                    <root>
                        <one/>
                        <two/>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            /// More elements than expected size of sequence, each element has
            /// the same name. Failure expected. If you wish to ignore excess
            /// elements, use the special type, that consume as much elements
            /// as possible, but ignores excess elements
            #[test]
            fn excess_elements() {
                from_str::<List>(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                        <four/>
                    </root>
                    "#,
                )
                .unwrap_err();
            }

            #[test]
            fn mixed_content() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [(); 3],
                }

                from_str::<List>(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                )
                .unwrap();
            }

            // There cannot be unknown items, because any tag name is accepted

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    #[serde(rename = "$value")]
                    item: [Choice; 3],
                    node: (),
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => {
                            assert_eq!(e, "invalid length 1, expected an array of length 3")
                        }
                        e => panic!(
                            r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;

                /// A field with a variable-name items defined before a field with a fixed-name
                /// items
                mod choice_and_fixed {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                        element: [(); 2],
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn overlapped_fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 2")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 2")), got {:?}"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn overlapped_fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }

                /// A field with a variable-name items defined after a field with a fixed-name
                /// items
                mod fixed_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        element: [(); 2],
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn overlapped_fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 2")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 2")), got {:?}"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn overlapped_fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }

                /// Tests are ignored, but exists to show a problem.
                /// May be it will be solved in the future
                mod choice_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: [Choice; 3],
                        // Actually, we cannot rename both fields to `$value`, which is now
                        // required to indicate, that field accepts elements with any name
                        #[serde(rename = "$value")]
                        element: [Choice2; 2],
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn splitted() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <first/>
                                <second/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [Choice2::First, Choice2::Second],
                            }
                        );
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn overlapped() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <first/>
                                <two/>
                                <second/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: [Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: [Choice2::First, Choice2::Second],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: [Choice3; 3],
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <one>41</one>
                        <two>42</two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: [
                            Choice3::One(41),
                            Choice3::Two("42".to_string()),
                            Choice3::Other,
                        ],
                    }
                );

                from_str::<List>(
                    r#"
                    <root>
                        <one>41</one>
                        <two><item>42</item></two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap_err();
            }
        }

        /// This module contains tests where size of the list have an unspecified size
        mod variable_size {
            use super::*;
            use pretty_assertions::assert_eq;

            #[derive(Debug, PartialEq, Deserialize)]
            struct List {
                #[serde(rename = "$value")]
                item: Vec<Choice>,
            }

            /// Simple case: count of elements matches expected size of sequence,
            /// each element has the same name. Successful deserialization expected
            #[test]
            fn simple() {
                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                        <two/>
                        <three/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                    }
                );
            }

            /// Special case: empty sequence
            #[test]
            #[ignore = "it is impossible to distinguish between missed field and empty list: use `Option<>` or #[serde(default)]"]
            fn empty() {
                let data = from_str::<List>(r#"<root></root>"#).unwrap();
                assert_eq!(data, List { item: vec![] });

                let data = from_str::<List>(r#"<root/>"#).unwrap();
                assert_eq!(data, List { item: vec![] });
            }

            /// Special case: one-element sequence
            #[test]
            fn one_element() {
                let data: List = from_str(
                    r#"
                    <root>
                        <one/>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![Choice::One],
                    }
                );
            }

            #[test]
            fn mixed_content() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: Vec<()>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <element/>
                        text
                        <![CDATA[cdata]]>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![(), (), ()],
                    }
                );
            }

            // There cannot be unknown items, because any tag name is accepted

            /// In those tests non-sequential field is defined in the struct
            /// before sequential, so it will be deserialized before the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_before_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    node: (),
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            node: (),
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            node: (),
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests non-sequential field is defined in the struct
            /// after sequential, so it will be deserialized after the list.
            /// That struct should be deserialized from the XML where these
            /// fields comes in an arbitrary order
            mod field_after_list {
                use super::*;
                use pretty_assertions::assert_eq;

                #[derive(Debug, PartialEq, Deserialize)]
                struct Root {
                    #[serde(rename = "$value")]
                    item: Vec<Choice>,
                    node: (),
                }

                #[test]
                fn before() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <node/>
                            <one/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn after() {
                    let data: Root = from_str(
                        r#"
                        <root>
                            <one/>
                            <two/>
                            <three/>
                            <node/>
                        </root>
                        "#,
                    )
                    .unwrap();

                    assert_eq!(
                        data,
                        Root {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );
                }

                #[test]
                fn overlapped() {
                    let data = from_str::<Root>(
                        r#"
                        <root>
                            <one/>
                            <node/>
                            <two/>
                            <three/>
                        </root>
                        "#,
                    );

                    #[cfg(feature = "overlapped-lists")]
                    assert_eq!(
                        data.unwrap(),
                        Root {
                            item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            node: (),
                        }
                    );

                    #[cfg(not(feature = "overlapped-lists"))]
                    match data {
                        Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                        e => panic!(
                            r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                            e
                        ),
                    }
                }
            }

            /// In those tests two lists are deserialized simultaneously.
            /// Lists should be deserialized even when them overlaps
            mod two_lists {
                use super::*;

                /// A field with a variable-name items defined before a field with a fixed-name
                /// items
                mod choice_and_fixed {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                        element: Vec<()>,
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn overlapped_fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `element`"),
                            e => panic!(
                                r#"Expected Err(Custom("duplicate field `element`")), got {:?}"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn overlapped_fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![(), ()],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                            e => panic!(
                                r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }

                /// A field with a variable-name items defined after a field with a fixed-name
                /// items
                mod fixed_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        element: Vec<()>,
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                    }

                    /// A list with fixed-name elements located before a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_before() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <element/>
                                <element/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }

                    /// A list with fixed-name elements located after a list with variable-name
                    /// elements in an XML
                    #[test]
                    fn fixed_after() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <one/>
                                <two/>
                                <three/>
                                <element/>
                                <element/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a fixed-name one
                    #[test]
                    fn overlapped_fixed_before() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <element/>
                                <one/>
                                <two/>
                                <element/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `element`"),
                            e => panic!(
                                r#"Expected Err(Custom("duplicate field `element`")), got {:?}"#,
                                e
                            ),
                        }
                    }

                    /// A list with fixed-name elements are mixed with a list with variable-name
                    /// elements in an XML, and the first element is a variable-name one
                    #[test]
                    fn overlapped_fixed_after() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <element/>
                                <two/>
                                <three/>
                                <element/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                element: vec![(), ()],
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => assert_eq!(e, "duplicate field `$value`"),
                            e => panic!(
                                r#"Expected Err(Custom("duplicate field `$value`")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }

                /// Tests are ignored, but exists to show a problem.
                /// May be it will be solved in the future
                mod choice_and_choice {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[derive(Debug, PartialEq, Deserialize)]
                    struct Pair {
                        #[serde(rename = "$value")]
                        item: Vec<Choice>,
                        // Actually, we cannot rename both fields to `$value`, which is now
                        // required to indicate, that field accepts elements with any name
                        #[serde(rename = "$value")]
                        element: Vec<Choice2>,
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn splitted() {
                        let data: Pair = from_str(
                            r#"
                            <root>
                                <first/>
                                <second/>
                                <one/>
                                <two/>
                                <three/>
                            </root>
                            "#,
                        )
                        .unwrap();

                        assert_eq!(
                            data,
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![Choice2::First, Choice2::Second],
                            }
                        );
                    }

                    #[test]
                    #[ignore = "There is no way to associate XML elements with `item` or `element` without extra knowledge from type"]
                    fn overlapped() {
                        let data = from_str::<Pair>(
                            r#"
                            <root>
                                <one/>
                                <first/>
                                <two/>
                                <second/>
                                <three/>
                            </root>
                            "#,
                        );

                        #[cfg(feature = "overlapped-lists")]
                        assert_eq!(
                            data.unwrap(),
                            Pair {
                                item: vec![Choice::One, Choice::Two, Choice::Other("three".into())],
                                element: vec![Choice2::First, Choice2::Second],
                            }
                        );

                        #[cfg(not(feature = "overlapped-lists"))]
                        match data {
                            Err(DeError::Custom(e)) => {
                                assert_eq!(e, "invalid length 1, expected an array of length 3")
                            }
                            e => panic!(
                                r#"Expected Err(Custom("invalid length 1, expected an array of length 3")), got {:?}"#,
                                e
                            ),
                        }
                    }
                }
            }

            /// Deserialization of primitives slightly differs from deserialization
            /// of complex types, so need to check this separately
            #[test]
            fn primitives() {
                #[derive(Debug, PartialEq, Deserialize)]
                struct List {
                    #[serde(rename = "$value")]
                    item: Vec<Choice3>,
                }

                let data: List = from_str(
                    r#"
                    <root>
                        <one>41</one>
                        <two>42</two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap();

                assert_eq!(
                    data,
                    List {
                        item: vec![
                            Choice3::One(41),
                            Choice3::Two("42".to_string()),
                            Choice3::Other,
                        ],
                    }
                );

                from_str::<List>(
                    r#"
                    <root>
                        <one>41</one>
                        <two><item>42</item></two>
                        <three>43</three>
                    </root>
                    "#,
                )
                .unwrap_err();
            }
        }
    }
}

macro_rules! maplike_errors {
    ($type:ty) => {
        mod non_closed {
            use super::*;

            #[test]
            fn attributes() {
                let data = from_str::<$type>(r#"<root float="42" string="answer">"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$type>(r#"<root float="42"><string>answer</string>"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }

            #[test]
            fn elements_child() {
                let data = from_str::<$type>(r#"<root float="42"><string>answer"#);

                match data {
                    Err(DeError::UnexpectedEof) => (),
                    _ => panic!("Expected `UnexpectedEof`, found {:?}", data),
                }
            }
        }

        mod mismatched_end {
            use super::*;
            use fast_xml::Error::EndEventMismatch;

            #[test]
            fn attributes() {
                let data = from_str::<$type>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer"></mismatched>"#,
                );

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                }
            }

            #[test]
            fn elements_root() {
                let data = from_str::<$type>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</string></mismatched>"#,
                );

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                }
            }

            #[test]
            fn elements_child() {
                let data = from_str::<$type>(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42"><string>answer</mismatched></root>"#,
                );

                match data {
                    Err(DeError::InvalidXml(EndEventMismatch { .. })) => (),
                    _ => panic!("Expected `InvalidXml(EndEventMismatch)`, found {:?}", data),
                }
            }
        }
    };
}

mod map {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn elements() {
        let data: HashMap<(), ()> = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
        );
    }

    #[test]
    fn attributes() {
        let data: HashMap<(), ()> = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
        );
    }

    #[test]
    fn attribute_and_element() {
        let data: HashMap<(), ()> = from_str(
            r#"
            <root float="42">
                <string>answer</string>
            </root>
            "#,
        )
        .unwrap();

        assert_eq!(
            data,
            HashMap::from_iter([((), ()), ((), ()),].iter().cloned())
        );
    }

    maplike_errors!(HashMap<(), ()>);
}

mod struct_ {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Struct {
        float: f64,
        string: String,
    }

    #[test]
    fn elements() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn excess_elements() {
        let data: Struct = from_str(
            r#"
            <root>
                <before/>
                <float>42</float>
                <in-the-middle/>
                <string>answer</string>
                <after/>
            </root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn excess_attributes() {
        let data: Struct = from_str(
            r#"<root before="1" float="42" in-the-middle="2" string="answer" after="3"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attribute_and_element() {
        let data: Struct = from_str(
            r#"
            <root float="42">
                <string>answer</string>
            </root>
        "#,
        )
        .unwrap();

        assert_eq!(
            data,
            Struct {
                float: 42.0,
                string: "answer".into()
            }
        );
    }

    maplike_errors!(Struct);
}

mod nested_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Struct {
        nested: Nested,
        string: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        float: f32,
    }

    #[test]
    fn elements() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: 42.0 },
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root string="answer"><nested float="42"/></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: 42.0 },
                string: "answer".into()
            }
        );
    }
}

mod flatten_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Struct {
        #[serde(flatten)]
        nested: Nested,
        string: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
        float: String,
    }

    #[test]
    #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
    fn elements() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root><float>42</float><string>answer</string></root>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: "42".into() },
                string: "answer".into()
            }
        );
    }

    #[test]
    fn attributes() {
        let data: Struct = from_str(
            // Comment for prevent unnecessary formatting - we use the same style in all tests
            r#"<root float="42" string="answer"/>"#,
        )
        .unwrap();
        assert_eq!(
            data,
            Struct {
                nested: Nested { float: "42".into() },
                string: "answer".into()
            }
        );
    }
}

mod enum_ {
    use super::*;

    mod externally_tagged {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, Deserialize, PartialEq)]
        enum Node {
            Unit,
            Newtype(bool),
            //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                float: f64,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            },
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        enum Workaround {
            Tuple(f64, String),
        }

        #[test]
        fn unit() {
            let data: Node = from_str("<Unit/>").unwrap();
            assert_eq!(data, Node::Unit);
        }

        #[test]
        fn newtype() {
            let data: Node = from_str("<Newtype>true</Newtype>").unwrap();
            assert_eq!(data, Node::Newtype(true));
        }

        #[test]
        fn tuple_struct() {
            let data: Workaround = from_str("<Tuple>42</Tuple><Tuple>answer</Tuple>").unwrap();
            assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Struct><float>42</float><string>answer</string></Struct>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Struct float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    r#"<Holder><string>answer</string><nested><float>42</float></nested></Holder>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Holder string="answer"><nested float="42"/></Holder>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }

        mod flatten_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Flatten><float>42</float><string>answer</string></Flatten>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<Flatten float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }
    }

    mod internally_tagged {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag")]
        enum Node {
            Unit,
            /// Primitives (such as `bool`) are not supported by serde in the internally tagged mode
            Newtype(NewtypeContent),
            // Tuple(f64, String),// Tuples are not supported in the internally tagged mode
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            Struct {
                float: String,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            },
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct NewtypeContent {
            value: bool,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        mod unit {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(r#"<root tag="Unit"/>"#).unwrap();
                assert_eq!(data, Node::Unit);
            }
        }

        mod newtype {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><tag>Newtype</tag><value>true</value></root>"#,
                )
                .unwrap();
                assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Newtype" value="true"/>"#,
                )
                .unwrap();
                assert_eq!(data, Node::Newtype(NewtypeContent { value: true }));
            }
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Struct</tag><float>42</float><string>answer</string></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: "42".into(),
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Struct" float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: "42".into(),
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Holder</tag><string>answer</string><nested><float>42</float></nested></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Holder" string="answer"><nested float="42"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }

        mod flatten_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Flatten</tag><float>42</float><string>answer</string></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Flatten" float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }
    }

    mod adjacently_tagged {
        use super::*;

        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag", content = "content")]
        enum Node {
            Unit,
            Newtype(bool),
            //TODO: serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                float: f64,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                string: String,
            },
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "tag", content = "content")]
        enum Workaround {
            Tuple(f64, String),
        }

        mod unit {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(r#"<root><tag>Unit</tag></root>"#).unwrap();
                assert_eq!(data, Node::Unit);
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(r#"<root tag="Unit"/>"#).unwrap();
                assert_eq!(data, Node::Unit);
            }
        }

        mod newtype {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><tag>Newtype</tag><content>true</content></root>"#,
                )
                .unwrap();
                assert_eq!(data, Node::Newtype(true));
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(r#"<root tag="Newtype" content="true"/>"#).unwrap();
                assert_eq!(data, Node::Newtype(true));
            }
        }

        mod tuple_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Workaround = from_str(
                    r#"<root><tag>Tuple</tag><content>42</content><content>answer</content></root>"#,
                ).unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn attributes() {
                let data: Workaround = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Tuple" content="42"><content>answer</content></root>"#,
                )
                .unwrap();
                assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
            }
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Struct</tag><content><float>42</float><string>answer</string></content></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Struct"><content float="42" string="answer"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn elements() {
                let data: Node = from_str(
                    r#"<root>
                        <tag>Holder</tag>
                        <content>
                            <string>answer</string>
                            <nested>
                                <float>42</float>
                            </nested>
                        </content>
                    </root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    r#"<root tag="Holder"><content string="answer"><nested float="42"/></content></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }

        mod flatten_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><tag>Flatten</tag><content><float>42</float><string>answer</string></content></root>"#,
                ).unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root tag="Flatten"><content float="42" string="answer"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }
    }

    mod untagged {
        use super::*;
        use pretty_assertions::assert_eq;

        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum Node {
            Unit,
            Newtype(bool),
            // serde bug https://github.com/serde-rs/serde/issues/1904
            // Tuple(f64, String),
            Struct {
                float: f64,
                string: String,
            },
            Holder {
                nested: Nested,
                string: String,
            },
            Flatten {
                #[serde(flatten)]
                nested: Nested,
                // Can't use "string" as name because in that case this variant
                // will have no difference from `Struct` variant
                string2: String,
            },
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Nested {
            //TODO: change to f64 after fixing https://github.com/serde-rs/serde/issues/1183
            float: String,
        }

        /// Workaround for serde bug https://github.com/serde-rs/serde/issues/1904
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum Workaround {
            Tuple(f64, String),
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn unit() {
            // Unit variant consists just from the tag, and because tags
            // are not written, nothing is written
            let data: Node = from_str("").unwrap();
            assert_eq!(data, Node::Unit);
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn newtype() {
            let data: Node = from_str("true").unwrap();
            assert_eq!(data, Node::Newtype(true));
        }

        #[test]
        #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
        fn tuple_struct() {
            let data: Workaround = from_str("<root>42</root><root>answer</root>").unwrap();
            assert_eq!(data, Workaround::Tuple(42.0, "answer".into()));
        }

        mod struct_ {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><float>42</float><string>answer</string></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Struct {
                        float: 42.0,
                        string: "answer".into()
                    }
                );
            }
        }

        mod nested_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    r#"<root><string>answer</string><nested><float>42</float></nested></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root string="answer"><nested float="42"/></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Holder {
                        nested: Nested { float: "42".into() },
                        string: "answer".into()
                    }
                );
            }
        }

        mod flatten_struct {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            #[ignore = "Prime cause: deserialize_any under the hood + https://github.com/serde-rs/serde/issues/1183"]
            fn elements() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root><float>42</float><string2>answer</string2></root>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string2: "answer".into()
                    }
                );
            }

            #[test]
            fn attributes() {
                let data: Node = from_str(
                    // Comment for prevent unnecessary formatting - we use the same style in all tests
                    r#"<root float="42" string2="answer"/>"#,
                )
                .unwrap();
                assert_eq!(
                    data,
                    Node::Flatten {
                        nested: Nested { float: "42".into() },
                        string2: "answer".into()
                    }
                );
            }
        }
    }
}
