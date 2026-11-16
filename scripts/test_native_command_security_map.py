import unittest

from check_native_command_security_map import (
    command_tokens,
    find_matching_delimiter,
    missing_demo_command_guards,
    missing_file_metadata_guards,
    missing_document_ipc_preflight_guards,
    missing_history_receipt_guards,
    missing_history_listing_receipt_guards,
    missing_history_key_recovery_guards,
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

    def test_character_literals_are_masked_without_masking_lifetimes(self):
        source = """
        fn unrelated() {
            let quote = '"';
        }
        fn guarded<'a>(value: &'a str) {
            if !debug_demo_commands_enabled() { return; }
            let _ = value;
        }
        """
        cleaned = __import__("check_native_command_security_map").strip_rust_noncode(source)
        self.assertNotIn('"', cleaned)
        self.assertIn("&'a str", cleaned)
        self.assertEqual(
            missing_demo_command_guards(
                source + """
                fn list_local_models() { if !debug_demo_commands_enabled() { return; } }
                fn run_local_model_task() { if !debug_demo_commands_enabled() { return; } }
                """
            ),
            ["run_local_task"],
        )

    def test_unterminated_comment_fails_closed(self):
        with self.assertRaises(ValueError):
            find_matching_delimiter("call(/* never closes", 4, "(", ")")

    def test_commented_fake_function_cannot_mask_unguarded_command(self):
        source = """
        // fn run_local_task() {
        //     if !debug_demo_commands_enabled() { return; }
        // }
        fn run_local_task() {
            return;
        }
        """
        self.assertEqual(missing_demo_command_guards(source), [
            "list_local_models",
            "run_local_model_task",
            "run_local_task",
        ])

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

    def test_selected_file_metadata_commands_must_use_the_core_validators(self):
        source = """
        fn analyze_text_file() {
            LocalTaskEngine::analyze_text(file_name, content)
        }
        fn inspect_image_file() {
            LocalTaskEngine::inspect_image(file_name, data, file_size_bytes)
        }
        """
        self.assertEqual(missing_file_metadata_guards(source), [])
        altered = source.replace("LocalTaskEngine::inspect_image", "Ok")
        self.assertEqual(missing_file_metadata_guards(altered), ["inspect_image_file"])


    def test_history_listing_maintenance_is_receipt_wrapped(self):
        source = """
        fn list_task_history() {
            with_local_state_receipt(
                "history.entry.list",
                b"history.entry.list-maintenance.v1",
                || {
                    migrate_plaintext_rows(&db, &key)?;
                    db.execute("DELETE FROM task_history WHERE created_at < ?1", [])?;
                    let entries = list_secure(&db, &key)?;
                    let output_metadata = serde_json::to_vec(&entries.len())?;
                    Ok((entries, output_metadata))
                },
            )
        }
        """
        self.assertEqual(missing_history_listing_receipt_guards(source), [])
        unwrapped = source.replace("with_local_state_receipt(", "execute_without_receipt(")
        self.assertEqual(missing_history_listing_receipt_guards(unwrapped), ["list_task_history"])
        content_bearing = source.replace("serde_json::to_vec(&entries.len())", "serde_json::to_vec(&entries)")
        self.assertEqual(missing_history_listing_receipt_guards(content_bearing), ["list_task_history"])

    def test_history_receipt_is_metadata_only_and_validation_precedes_reservation(self):
        source = """
        fn save_task_history() {
            validate_history_values(prompt, output, mode, model, kind)?;
            let input = history_receipt_metadata(mode, model, kind, prompt.len(), output.len())?;
            with_local_state_receipt(input);
        }
        fn history_receipt_metadata(
            mode: &str, model: &str, kind: &str,
            prompt_bytes: usize, output_bytes: usize,
        ) {
            serde_json::to_vec(&(mode, model, kind, prompt_bytes, output_bytes))
        }
        """
        self.assertEqual(missing_history_receipt_guards(source), [])
        late_validation = source.replace("validate_history_values", "sha256_hex")
        self.assertEqual(missing_history_receipt_guards(late_validation), ["save_task_history"])
        content_bearing_metadata = source.replace("prompt_bytes", "prompt")
        self.assertEqual(missing_history_receipt_guards(content_bearing_metadata), ["history_receipt_metadata"])


    def test_document_ipc_preflight_precedes_registration_runtime_and_decode(self):
        source = """
        fn execute_document_workflow() {
            task_engine::validate_document_request(file_name, content, mode)?;
            register_active_task();
        }
        fn process_selected_document() {
            task_engine::validate_document_request(file_name, content, mode)?;
            register_active_task();
            get_capability_snapshot_for_task();
        }
        fn process_selected_pdf() {
            task_engine::validate_pdf_metadata(file_name, mode)?;
            validate_bounded_pdf_encoded_length(data_base64, MAX_PDF_UPLOAD_BYTES)?;
            register_active_task();
            get_capability_snapshot_for_task();
            decode_pdf_upload_off_thread();
        }
        """
        self.assertEqual(missing_document_ipc_preflight_guards(source), [])
        late_pdf_check = source.replace(
            "task_engine::validate_pdf_metadata(file_name, mode)?;\n            validate_bounded_pdf_encoded_length(data_base64, MAX_PDF_UPLOAD_BYTES)?;\n            register_active_task();",
            "validate_bounded_pdf_encoded_length(data_base64, MAX_PDF_UPLOAD_BYTES)?;\n            register_active_task();\n            task_engine::validate_pdf_metadata(file_name, mode)?;",
        )
        self.assertEqual(
            missing_document_ipc_preflight_guards(late_pdf_check),
            ["process_selected_pdf"],
        )


    def test_selected_file_metadata_commands_must_use_the_core_validators(self):
        source = """
        fn analyze_text_file() {
            LocalTaskEngine::analyze_text(file_name, content)
        }
        fn inspect_image_file() {
            LocalTaskEngine::inspect_image(file_name, data, file_size_bytes)
        }
        """
        self.assertEqual(missing_file_metadata_guards(source), [])
        altered = source.replace("LocalTaskEngine::inspect_image", "Ok")
        self.assertEqual(missing_file_metadata_guards(altered), ["inspect_image_file"])


    def test_missing_history_key_does_not_get_silently_replaced(self):
        source = """
        fn history_has_rows(db) {}
        fn history_key_was_initialized(db) {}
        fn mark_history_key_initialized(db) {}
        fn history_has_encrypted_rows(db) {}
        fn missing_history_key_error(has_rows, key_was_initialized, has_encrypted_rows) {}
        fn history_key(db) {
            let has_rows = history_has_rows(db)?;
            let key_was_initialized = history_key_was_initialized(db)?;
            let has_encrypted_rows = history_has_encrypted_rows(db)?;
            if let Some(message) = missing_history_key_error(has_rows, key_was_initialized, has_encrypted_rows) {
                return Err(message.into());
            }
            entry.set_password(encoded.as_str())?;
            mark_history_key_initialized(db)?;
        }
        fn call_sites() {
            history_key(&db)?;
            history_key(&db)?;
            history_key(&db)?;
        }
        """
        self.assertEqual(missing_history_key_recovery_guards(source), [])
        unsafe = source.replace(
            "if let Some(message) = missing_history_key_error(has_rows, key_was_initialized, has_encrypted_rows) {\n                return Err(message.into());\n            }\n            entry.set_password(encoded.as_str())?;",
            "entry.set_password(encoded.as_str())?;\n            if let Some(message) = missing_history_key_error(has_rows, key_was_initialized, has_encrypted_rows) {\n                return Err(message.into());\n            }",
        )
        self.assertNotEqual(unsafe, source)
        self.assertEqual(missing_history_key_recovery_guards(unsafe), ["history_key"])


if __name__ == "__main__":
    unittest.main()
