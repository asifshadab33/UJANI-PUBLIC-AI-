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
    missing_chat_ipc_preflight_guards,
    registered_commands,
    missing_cloud_transfer_preflight_guards,
    missing_folder_import_preflight_guards,
    missing_local_state_mutation_receipt_guards,
    manifest_entry_errors,
)


class NativeCommandSideEffectOrderingTests(unittest.TestCase):
    def test_cloud_policy_and_native_approval_precede_receipt_budget_and_network(self):
        source = """
        async fn execute_cloud_ai_task() {
            ujani_core::policy_gate::parse_data_class();
            ujani_core::policy_gate::evaluate_registered_external_transfer();
            cloud_gateway::validate_request();
            native_consent::confirm_native().await;
            reserve_execution_receipt_slot();
            reserve_cloud_tokens();
            cloud_gateway::execute().await;
        }
        """
        self.assertEqual(missing_cloud_transfer_preflight_guards(source), [])
        unsafe = source.replace(
            "native_consent::confirm_native().await;",
            "cloud_gateway::execute().await; native_consent::confirm_native().await;",
        )
        self.assertEqual(missing_cloud_transfer_preflight_guards(unsafe), ["execute_cloud_ai_task"])

    def test_selected_folder_capability_gate_precedes_native_picker_and_walk(self):
        source = """
        async fn process_selected_folder() {
            register_active_task();
            get_capability_snapshot_for_task().await;
            require_local_operation();
            blocking_pick_folder();
            collect_selected_folder();
        }
        """
        self.assertEqual(missing_folder_import_preflight_guards(source), [])
        unsafe = source.replace(
            "require_local_operation();\n            blocking_pick_folder();",
            "blocking_pick_folder();\n            require_local_operation();",
        )
        self.assertEqual(missing_folder_import_preflight_guards(unsafe), ["process_selected_folder"])

    def test_folder_import_does_not_accept_a_webview_path(self):
        source = """
        async fn process_selected_folder() {
            register_active_task();
            get_capability_snapshot_for_task().await;
            require_local_operation();
            let selected_path = PathBuf::from("caller-controlled");
            blocking_pick_folder();
            collect_selected_folder();
        }
        """
        self.assertEqual(missing_folder_import_preflight_guards(source), ["process_selected_folder"])



class LocalChatPreflightOrderingTests(unittest.TestCase):
    def test_chat_input_is_rejected_before_task_registration_and_local_side_effects(self):
        source = """
        async fn execute_local_ai_task() {
            ollama::validate_chat_input(&model, &prompt)?;
            register_active_task();
            get_capability_snapshot_for_task().await;
            ollama::list_models();
            MemoryReservation::acquire_queued().await;
            reserve_execution_receipt_slot();
        }
        """
        self.assertEqual(missing_chat_ipc_preflight_guards(source), [])
        late_validation = source.replace(
            "ollama::validate_chat_input(&model, &prompt)?;\n            register_active_task();",
            "register_active_task();\n            ollama::validate_chat_input(&model, &prompt)?;",
        )
        self.assertEqual(missing_chat_ipc_preflight_guards(late_validation), ["execute_local_ai_task"])

    def test_chat_validator_in_comment_cannot_satisfy_executable_order(self):
        source = """
        async fn execute_local_ai_task() {
            // ollama::validate_chat_input(&model, &prompt)?;
            register_active_task();
            get_capability_snapshot_for_task().await;
            ollama::list_models();
            MemoryReservation::acquire_queued().await;
            reserve_execution_receipt_slot();
        }
        """
        self.assertEqual(missing_chat_ipc_preflight_guards(source), ["execute_local_ai_task"])


class LocalStateMutationReceiptOrderingTests(unittest.TestCase):
    def test_cache_mutations_reserve_receipt_before_transactional_writes(self):
        cache = """
        fn set_local_search_cache_settings() {
            with_local_state_receipt(|| {
                transaction_with_behavior();
                UPDATE cache_settings;
                DELETE FROM search_cache;
                DELETE FROM recent_searches;
                DELETE FROM approved_scopes;
            });
        }
        fn clear_local_search_cache() {
            with_local_state_receipt(|| {
                transaction_with_behavior();
                DELETE FROM search_cache;
                DELETE FROM recent_searches;
                DELETE FROM approved_scopes;
            });
        }
        """
        history = """
        fn set_task_history_enabled() {
            if enabled_value {
                confirm_native().await;
                if !approved { return; }
            }
            with_local_state_receipt(|| {
                migrate_plaintext_rows();
                UPDATE history_settings;
            });
        }
        fn delete_task_history() {
            with_local_state_receipt(|| { DELETE FROM task_history; });
        }
        fn clear_task_history() {
            with_local_state_receipt(|| { DELETE FROM task_history; });
        }
        """
        security = """
        fn set_cloud_budget_limit() {
            confirm_native().await;
            if !approved { return; }
            with_local_state_receipt(|| { set_cloud_budget_limit_db(); });
        }
        """
        self.assertEqual(missing_local_state_mutation_receipt_guards(cache, history, security), [])

    def test_cache_clear_without_receipt_is_rejected(self):
        cache = """
        fn set_local_search_cache_settings() {
            with_local_state_receipt(|| {
                transaction_with_behavior(); UPDATE cache_settings;
                DELETE FROM search_cache; DELETE FROM recent_searches; DELETE FROM approved_scopes;
            });
        }
        fn clear_local_search_cache() {
            transaction_with_behavior(); DELETE FROM search_cache;
            DELETE FROM recent_searches; DELETE FROM approved_scopes;
        }
        """
        history = """
        fn set_task_history_enabled() { if enabled_value { confirm_native().await; if !approved { return; } } with_local_state_receipt(|| { migrate_plaintext_rows(); UPDATE history_settings; }); }
        fn delete_task_history() { with_local_state_receipt(|| { DELETE FROM task_history; }); }
        fn clear_task_history() { with_local_state_receipt(|| { DELETE FROM task_history; }); }
        """
        security = """
        fn set_cloud_budget_limit() { confirm_native(); if !approved { return; } with_local_state_receipt(|| { set_cloud_budget_limit_db(); }); }
        """
        self.assertEqual(missing_local_state_mutation_receipt_guards(cache, history, security), ["clear_local_search_cache"])

    def test_cloud_budget_change_requires_consent_before_receipt_and_write(self):
        cache = """
        fn set_local_search_cache_settings() { with_local_state_receipt(|| { transaction_with_behavior(); UPDATE cache_settings; DELETE FROM search_cache; DELETE FROM recent_searches; DELETE FROM approved_scopes; }); }
        fn clear_local_search_cache() { with_local_state_receipt(|| { transaction_with_behavior(); DELETE FROM search_cache; DELETE FROM recent_searches; DELETE FROM approved_scopes; }); }
        """
        history = """
        fn set_task_history_enabled() { if enabled_value { confirm_native().await; if !approved { return; } } with_local_state_receipt(|| { migrate_plaintext_rows(); UPDATE history_settings; }); }
        fn delete_task_history() { with_local_state_receipt(|| { DELETE FROM task_history; }); }
        fn clear_task_history() { with_local_state_receipt(|| { DELETE FROM task_history; }); }
        """
        security = """
        fn set_cloud_budget_limit() { with_local_state_receipt(|| { confirm_native(); set_cloud_budget_limit_db(); }); }
        """
        self.assertEqual(missing_local_state_mutation_receipt_guards(cache, history, security), ["set_cloud_budget_limit"])

    def test_history_enable_requires_native_consent_before_receipt_and_migration(self):
        cache = """
        fn set_local_search_cache_settings() { with_local_state_receipt(|| { transaction_with_behavior(); UPDATE cache_settings; DELETE FROM search_cache; DELETE FROM recent_searches; DELETE FROM approved_scopes; }); }
        fn clear_local_search_cache() { with_local_state_receipt(|| { transaction_with_behavior(); DELETE FROM search_cache; DELETE FROM recent_searches; DELETE FROM approved_scopes; }); }
        """
        history = """
        fn set_task_history_enabled() {
            if enabled_value {
                confirm_native().await;
                if !approved { return; }
            }
            with_local_state_receipt(|| {
                migrate_plaintext_rows();
                UPDATE history_settings;
            });
        }
        fn delete_task_history() { with_local_state_receipt(|| { DELETE FROM task_history; }); }
        fn clear_task_history() { with_local_state_receipt(|| { DELETE FROM task_history; }); }
        """
        security = """
        fn set_cloud_budget_limit() {
            confirm_native().await;
            if !approved { return; }
            with_local_state_receipt(|| { set_cloud_budget_limit_db(); });
        }
        """
        self.assertEqual(missing_local_state_mutation_receipt_guards(cache, history, security), [])
        consent_after_receipt = history.replace(
            "confirm_native().await;",
            "with_local_state_receipt(); confirm_native().await;",
        )
        self.assertEqual(
            missing_local_state_mutation_receipt_guards(cache, consent_after_receipt, security),
            ["set_task_history_enabled"],
        )
        without_consent = history.replace("confirm_native().await;", "skip_confirmation();")
        self.assertEqual(
            missing_local_state_mutation_receipt_guards(cache, without_consent, security),
            ["set_task_history_enabled"],
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

class NativeCommandManifestEnumTests(unittest.TestCase):
    def test_known_read_only_mapping_is_accepted(self):
        entry = {
            "classification": "read_only_local",
            "receipt": "none-read-only",
            "status": "source-mapped; full review pending",
        }
        self.assertEqual(manifest_entry_errors("default_commands", "get_capability_snapshot", entry), [])

    def test_unknown_classification_and_receipt_are_rejected(self):
        entry = {
            "classification": "admin_magic",
            "receipt": "optional-maybe",
            "status": "source-mapped; full review pending",
        }
        errors = manifest_entry_errors("default_commands", "x", entry)
        self.assertTrue(any("unknown classification" in error for error in errors))
        self.assertTrue(any("unknown receipt state" in error for error in errors))

    def test_mutating_command_cannot_claim_no_receipt(self):
        entry = {
            "classification": "persistent_local_mutation",
            "receipt": "none-read-only",
            "status": "source-mapped; full review pending",
        }
        errors = manifest_entry_errors("default_commands", "clear_state", entry)
        self.assertTrue(any("mutating command cannot" in error for error in errors))

    def test_peer_preview_commands_must_keep_their_feature_gate_label(self):
        entry = {
            "classification": "peer_trust_mutation",
            "receipt": "metadata-outcome-receipt",
            "status": "full review pending",
        }
        errors = manifest_entry_errors("peer_preview_commands", "enroll", entry)
        self.assertTrue(any("peer-preview-only status" in error for error in errors))

    def test_default_command_cannot_be_mislabelled_peer_preview_only(self):
        entry = {
            "classification": "read_only_local",
            "receipt": "none-read-only",
            "status": "peer-preview only; full review pending",
        }
        errors = manifest_entry_errors("default_commands", "x", entry)
        self.assertTrue(any("cannot be labelled peer-preview only" in error for error in errors))

