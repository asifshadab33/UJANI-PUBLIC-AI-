import unittest

from check_native_command_security_map import (
    command_tokens,
    find_matching_delimiter,
    missing_demo_command_guards,
    registered_commands,
)


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

    def test_each_demo_command_guard_is_checked_independently(self):
        source = """
        fn list_local_models() {
            if !debug_demo_commands_enabled() { return; }
        }
        fn run_local_model_task() {
            if !debug_demo_commands_enabled() { return; }
        }
        fn run_local_task() {
            if !debug_demo_commands_enabled() { return; }
        }
        """
        self.assertEqual(missing_demo_command_guards(source), [])

        for name in ("list_local_models", "run_local_model_task", "run_local_task"):
            guarded = (
                f"        fn {name}() {{\n"
                "            if !debug_demo_commands_enabled() { return; }\n"
                "        }"
            )
            unguarded = f"fn {name}() {{ return; }}"
            altered = source.replace(guarded, unguarded)
            self.assertNotEqual(altered, source, f"test fixture did not remove {name} guard")
            self.assertEqual(missing_demo_command_guards(altered), [name])


if __name__ == "__main__":
    unittest.main()
