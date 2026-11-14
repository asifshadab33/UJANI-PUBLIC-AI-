import unittest

from check_native_command_security_map import command_tokens, find_matching_delimiter, registered_commands


class NativeCommandSecurityMapParserTests(unittest.TestCase):
    def test_square_bracket_in_line_comment_does_not_truncate_handler(self):
        source = """tauri::generate_handler![
            alpha,
            // A comment with a fake close bracket: ]
            beta,
        ];
        #[cfg(feature = "peer-preview")]
        let builder = builder.invoke_handler(ujani_invoke_handler!(
            peer::one,
            // A fake close parenthesis: )
            peer::two,
        ));
        """
        default, peer = registered_commands(source)
        self.assertEqual(default, ["alpha", "beta"])
        self.assertEqual(peer, ["peer::one", "peer::two"])

    def test_delimiters_inside_nested_block_comments_and_strings_are_ignored(self):
        source = 'call(["text ] )", /* outer ] /* nested ) */ still ] */ item])'
        opening = source.index("(")
        closing = find_matching_delimiter(source, opening, "(", ")")
        self.assertEqual(source[closing], ")")
        self.assertEqual(closing, len(source) - 1)

    def test_comment_markers_inside_line_comments_do_not_hide_commands(self):
        source = """alpha,
            // The text /* is not a block comment opener here: beta::hidden
            beta,
            /* outer comment
               /* nested comment with gamma::hidden */
               still a comment
            */
            gamma,
        """
        self.assertEqual(command_tokens(source), ["alpha", "beta", "gamma"])

    def test_commands_inside_strings_and_raw_strings_are_ignored(self):
        source = 'alpha, "fake::command", r#"other::fake"#, beta'
        self.assertEqual(command_tokens(source), ["alpha", "beta"])

    def test_unterminated_comment_fails_closed(self):
        with self.assertRaises(ValueError):
            find_matching_delimiter("call(/* never closes", 4, "(", ")")


if __name__ == "__main__":
    unittest.main()
