//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::constants::{DEFAULT_INSTALL_STRATEGY, UI_PATTERN_CATALOG_VERSION};
use super::model::{
    UiPatternAccessibility, UiPatternAdapter, UiPatternDescriptor, UiPatternEvidence,
    UiPatternInteraction, UiPatternQuality, UiPatternResponsive, UiPatternSpacing, UiPatternState,
    UiPatternTypography,
};
use super::validate::{
    default_certified_timestamp, default_verified_timestamp, epoch_record_time, required_states,
    validate_descriptor,
};

fn standard_typography() -> UiPatternTypography {
    UiPatternTypography {
        family: "system-ui".to_string(),
        scale: "1.125".to_string(),
        line_height: "1.5".to_string(),
        weight: "400".to_string(),
    }
}

fn standard_spacing() -> UiPatternSpacing {
    UiPatternSpacing {
        token: "design-token-spacing-4".to_string(),
        scale: "4,8,12,16,24,32".to_string(),
    }
}

fn standard_responsive() -> UiPatternResponsive {
    UiPatternResponsive {
        breakpoints: vec![
            "sm:640".to_string(),
            "md:768".to_string(),
            "lg:1024".to_string(),
        ],
        layout: "stack-on-mobile, side-by-side on >=md".to_string(),
    }
}

fn standard_accessibility() -> UiPatternAccessibility {
    UiPatternAccessibility {
        keyboard: "tab order follows visual order; Escape closes overlays".to_string(),
        focus: "first focusable element receives focus on mount".to_string(),
        aria: "role and aria-label follow the WAI-ARIA Authoring Practices".to_string(),
        contrast: "text/background pair meets WCAG AA (>=4.5:1)".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn descriptor(
    id: &str,
    version: &str,
    intent: &str,
    documentation: &str,
    extra_states: &[(&str, &str)],
    interaction_choices: &[&str],
    interaction_outcomes: &[&str],
    depends_on: &[&str],
    feature_deps: &[&str],
    adapters: Vec<UiPatternAdapter>,
    install_strategy: &str,
    tests: &str,
    quality: UiPatternQuality,
    evidence: UiPatternEvidence,
) -> UiPatternDescriptor {
    let mut states = required_states();
    for (name, description) in extra_states {
        states.push(UiPatternState {
            name: (*name).to_string(),
            description: (*description).to_string(),
        });
    }
    UiPatternDescriptor {
        id: id.to_string(),
        version: version.to_string(),
        intent: intent.to_string(),
        documentation: documentation.to_string(),
        states,
        typography: standard_typography(),
        spacing: standard_spacing(),
        responsive: standard_responsive(),
        accessibility: standard_accessibility(),
        interaction: UiPatternInteraction {
            choices: interaction_choices
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            outcomes: interaction_outcomes
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        },
        depends_on: depends_on.iter().map(|s| (*s).to_string()).collect(),
        feature_deps: feature_deps.iter().map(|s| (*s).to_string()).collect(),
        adapters,
        install_strategy: install_strategy.to_string(),
        tests: tests.to_string(),
        quality,
        evidence,
    }
}

fn verified_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 4,
        test_coverage: 0.78,
        last_verified: default_verified_timestamp(),
        known_issues: Vec::new(),
        security_review: false,
    }
}

fn certified_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 11,
        test_coverage: 0.92,
        last_verified: default_certified_timestamp(),
        known_issues: Vec::new(),
        security_review: true,
    }
}

fn experimental_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 1,
        test_coverage: 0.45,
        last_verified: epoch_record_time(),
        known_issues: vec!["initial draft, surface may change".to_string()],
        security_review: false,
    }
}

fn deprecated_evidence() -> UiPatternEvidence {
    UiPatternEvidence {
        usage_count: 0,
        test_coverage: 0.0,
        last_verified: epoch_record_time(),
        known_issues: vec!["deprecated in favor of the next generation".to_string()],
        security_review: false,
    }
}

fn react_adapter(pattern_id: &str, surface: &str) -> UiPatternAdapter {
    UiPatternAdapter {
        profile: "react-web".to_string(),
        surface: surface.to_string(),
        artifact_path: format!("src/ui/{pattern_id}.tsx"),
        tests: format!("tests/ui/{pattern_id}.test.tsx"),
        artifact_source: react_source(pattern_id),
    }
}

fn nextjs_adapter(pattern_id: &str, surface: &str) -> UiPatternAdapter {
    UiPatternAdapter {
        profile: "nextjs-web".to_string(),
        surface: surface.to_string(),
        artifact_path: format!("src/ui/{pattern_id}.tsx"),
        tests: format!("tests/ui/{pattern_id}.test.tsx"),
        artifact_source: nextjs_source(pattern_id),
    }
}

fn flutter_adapter(pattern_id: &str, surface: &str) -> UiPatternAdapter {
    UiPatternAdapter {
        profile: "flutter-app".to_string(),
        surface: surface.to_string(),
        artifact_path: format!("lib/ui/{pattern_id}.dart"),
        tests: format!("test/ui/{pattern_id}_test.dart"),
        artifact_source: flutter_source(pattern_id),
    }
}

fn react_source(pattern_id: &str) -> String {
    let cap = pattern_capitalize(pattern_id);
    let mut out = String::new();
    out.push_str("// Auto-generated by forge ui-pattern install (");
    out.push_str(pattern_id);
    out.push_str(")\n");
    out.push_str(
        "// This file is ordinary React source. Edit it freely; Forge never\n\
         // rewrites a customized file. After Forge is removed, the project\n\
         // continues to build with the native toolchain (npm run build).\n",
    );
    out.push_str("import { useState } from 'react';\n\n");
    out.push_str("export type ");
    out.push_str(&cap);
    out.push_str("Props = {\n    children?: React.ReactNode;\n};\n\n");
    out.push_str("export function ");
    out.push_str(&cap);
    out.push_str("({ children }: ");
    out.push_str(&cap);
    out.push_str("Props) {\n");
    out.push_str(
        "    const [state, setState] = useState<'idle' | 'loading' | 'error' | 'success'>('idle');\n",
    );
    out.push_str("    return (\n");
    out.push_str("        <section role=\"region\" aria-label=\"");
    out.push_str(pattern_id);
    out.push_str("\" data-state={state}>\n");
    out.push_str("            {children}\n");
    out.push_str("        </section>\n");
    out.push_str("    );\n}\n\n");
    out.push_str("export default ");
    out.push_str(&cap);
    out.push_str(";\n");
    out
}

fn nextjs_source(pattern_id: &str) -> String {
    let cap = pattern_capitalize(pattern_id);
    let mut out = String::new();
    out.push_str("// Auto-generated by forge ui-pattern install (");
    out.push_str(pattern_id);
    out.push_str(")\n");
    out.push_str(
        "// Ordinary Next.js source. Edit it freely; the project continues to\n\
         // build with the native toolchain (next build) after Forge is removed.\n",
    );
    out.push_str("import type { ReactNode } from 'react';\n\n");
    out.push_str("export type ");
    out.push_str(&cap);
    out.push_str("Props = {\n    children?: ReactNode;\n};\n\n");
    out.push_str("export function ");
    out.push_str(&cap);
    out.push_str("({ children }: ");
    out.push_str(&cap);
    out.push_str("Props) {\n");
    out.push_str("    return (\n");
    out.push_str("        <section role=\"region\" aria-label=\"");
    out.push_str(pattern_id);
    out.push_str("\">\n");
    out.push_str("            {children}\n");
    out.push_str("        </section>\n");
    out.push_str("    );\n}\n\n");
    out.push_str("export default ");
    out.push_str(&cap);
    out.push_str(";\n");
    out
}

fn flutter_source(pattern_id: &str) -> String {
    let cap = pattern_capitalize(pattern_id);
    let mut out = String::new();
    out.push_str("// Auto-generated by forge ui-pattern install (");
    out.push_str(pattern_id);
    out.push_str(")\n");
    out.push_str(
        "// Ordinary Flutter source. Edit it freely; the project continues to\n\
         // build with the native toolchain (flutter build) after Forge is removed.\n",
    );
    out.push_str("import 'package:flutter/material.dart';\n\n");
    out.push_str("class ");
    out.push_str(&cap);
    out.push_str(" extends StatelessWidget {\n");
    out.push_str("    const ");
    out.push_str(&cap);
    out.push_str("({super.key, this.child});\n\n");
    out.push_str("    final Widget? child;\n\n");
    out.push_str("    @override\n");
    out.push_str("    Widget build(BuildContext context) {\n");
    out.push_str("        return Semantics(\n");
    out.push_str("            label: '");
    out.push_str(pattern_id);
    out.push_str("',\n");
    out.push_str("            container: true,\n");
    out.push_str("            child: child ?? const SizedBox.shrink(),\n");
    out.push_str("        );\n");
    out.push_str("    }\n}\n");
    out
}

fn pattern_capitalize(id: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in id.chars() {
        if ch == '-' || ch == '_' {
            out.push(ch);
            upper = true;
            continue;
        }
        if upper {
            for upper_ch in ch.to_uppercase() {
                out.push(upper_ch);
            }
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// All catalog patterns in stable id order. Every entry has
/// been checked by [`validate_descriptor`] at module load
/// time.
pub fn ui_pattern_catalog() -> Vec<UiPatternDescriptor> {
    let mut out = vec![
        descriptor(
            "billing",
            UI_PATTERN_CATALOG_VERSION,
            "billing",
            "Billing surface with plan summary, payment method and invoice history.",
            &[("payment_method", "the stored payment method summary")],
            &["select_plan", "update_payment", "download_invoice"],
            &["plan_change", "payment_updated", "invoice_downloaded"],
            &["auth"],
            &["billing"],
            vec![
                react_adapter("billing", "react-jsx"),
                nextjs_adapter("billing", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::billing_lists_in_two_web_adapters",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "confirm-dialog",
            UI_PATTERN_CATALOG_VERSION,
            "confirm-dialog",
            "Destructive-action confirmation dialog with explicit user choice and a11y semantics.",
            &[("choice", "the explicit user choice (confirmed or cancelled)")],
            &["confirm", "cancel"],
            &["confirmed", "cancelled"],
            &["validated-form"],
            &[],
            vec![
                react_adapter("confirm-dialog", "react-jsx"),
                nextjs_adapter("confirm-dialog", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::confirm_dialog_declares_choice_state",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "crud-table",
            UI_PATTERN_CATALOG_VERSION,
            "crud-table",
            "CRUD table pattern with sorting, filtering, pagination and explicit state contracts.",
            &[
                ("sorting", "typed column sort key and direction"),
                ("filtering", "typed filter value per column"),
                ("pagination", "typed page request and cursor"),
            ],
            &["sort", "filter", "paginate", "select_row"],
            &["sorted", "filtered", "paged", "row_selected"],
            &["paginated-query", "validated-form"],
            &[],
            vec![
                react_adapter("crud-table", "react-jsx"),
                nextjs_adapter("crud-table", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::crud_table_exposes_sort_filter_paginate",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "dashboard",
            UI_PATTERN_CATALOG_VERSION,
            "dashboard",
            "Dashboard surface with KPI tiles, recent activity and a quick-action slot.",
            &[("recent_activity", "the recent-activity summary slot")],
            &["open_kpi", "trigger_quick_action"],
            &["kpi_opened", "quick_action_triggered"],
            &["auth"],
            &[],
            vec![
                react_adapter("dashboard", "react-jsx"),
                nextjs_adapter("dashboard", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::dashboard_lists_recent_activity",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "empty-state",
            UI_PATTERN_CATALOG_VERSION,
            "empty-state",
            "Empty-state block with a consistent call-to-action surface.",
            &[("call_to_action", "the call-to-action slot")],
            &["invoke_call_to_action"],
            &["call_to_action_invoked"],
            &[],
            &[],
            vec![
                react_adapter("empty-state", "react-jsx"),
                nextjs_adapter("empty-state", "nextjs-jsx"),
                flutter_adapter("empty-state", "flutter-widget"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::empty_state_supports_three_platforms",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "error-page",
            UI_PATTERN_CATALOG_VERSION,
            "error-page",
            "Full-page error surface with a recovery action and diagnostic details.",
            &[("diagnostics", "the typed diagnostic detail slot")],
            &["retry", "go_home"],
            &["retried", "navigated_home"],
            &[],
            &[],
            vec![
                react_adapter("error-page", "react-jsx"),
                nextjs_adapter("error-page", "nextjs-jsx"),
                flutter_adapter("error-page", "flutter-screen"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::error_page_lists_recovery_actions",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "file-upload",
            UI_PATTERN_CATALOG_VERSION,
            "file-upload",
            "Accessible file upload surface with accepted-type and size validation.",
            &[("upload_progress", "the per-file progress slot")],
            &["select_file", "cancel_upload"],
            &["file_selected", "upload_cancelled"],
            &[],
            &["storage"],
            vec![
                react_adapter("file-upload", "react-jsx"),
                nextjs_adapter("file-upload", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::file_upload_lists_progress_state",
            UiPatternQuality::Experimental,
            experimental_evidence(),
        ),
        descriptor(
            "filter-bar",
            UI_PATTERN_CATALOG_VERSION,
            "filter-bar",
            "Filter bar with typed filter chips, range inputs and a clear-all action.",
            &[("clear_all", "the explicit clear-all action")],
            &["apply_filter", "clear_filter", "clear_all"],
            &["filter_applied", "filter_cleared", "all_cleared"],
            &["paginated-query"],
            &[],
            vec![
                react_adapter("filter-bar", "react-jsx"),
                nextjs_adapter("filter-bar", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::filter_bar_lists_clear_all",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "forgot-password",
            UI_PATTERN_CATALOG_VERSION,
            "forgot-password",
            "Forgot-password flow with email entry, pending confirmation and recovery instructions.",
            &[("pending_recovery", "the pending recovery instructions slot")],
            &["submit_email"],
            &["email_submitted"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("forgot-password", "react-jsx"),
                nextjs_adapter("forgot-password", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::forgot_password_lists_pending_state",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "form",
            UI_PATTERN_CATALOG_VERSION,
            "form",
            "Form pattern with typed field validation, pending submit, failure and success states.",
            &[
                ("pending", "the typed pending submit surface"),
                ("failure", "the typed failure surface"),
            ],
            &["submit", "reset"],
            &["submitted", "reset"],
            &["validated-form"],
            &[],
            vec![
                react_adapter("form", "react-jsx"),
                nextjs_adapter("form", "nextjs-jsx"),
                flutter_adapter("form", "flutter-form"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::form_lists_pending_failure_success",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "login",
            UI_PATTERN_CATALOG_VERSION,
            "login",
            "Login surface with typed credentials, pending submit, failure and authenticated states.",
            &[
                ("pending", "the typed pending submit surface"),
                ("failure", "the typed failure surface"),
                ("authenticated", "the typed authenticated surface"),
            ],
            &["submit", "forgot_password", "register"],
            &["authenticated", "navigate_to_forgot_password", "navigate_to_register"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("login", "react-jsx"),
                nextjs_adapter("login", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::login_lists_authenticated_state",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "modal",
            UI_PATTERN_CATALOG_VERSION,
            "modal",
            "Modal overlay with focus trap, Escape-to-close and an explicit return focus target.",
            &[("return_focus", "the explicit return focus target")],
            &["close"],
            &["closed"],
            &[],
            &[],
            vec![
                react_adapter("modal", "react-jsx"),
                nextjs_adapter("modal", "nextjs-jsx"),
                flutter_adapter("modal", "flutter-dialog"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::modal_lists_return_focus",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "navigation",
            UI_PATTERN_CATALOG_VERSION,
            "navigation",
            "Primary navigation surface with typed destinations, active marker and skip-to-content link.",
            &[("skip_to_content", "the skip-to-content link")],
            &["navigate"],
            &["navigated"],
            &[],
            &[],
            vec![
                react_adapter("navigation", "react-jsx"),
                nextjs_adapter("navigation", "nextjs-jsx"),
                flutter_adapter("navigation", "flutter-drawer"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::navigation_lists_skip_to_content",
            UiPatternQuality::Certified,
            certified_evidence(),
        ),
        descriptor(
            "profile",
            UI_PATTERN_CATALOG_VERSION,
            "profile",
            "Profile surface with avatar, identity fields and explicit save and discard actions.",
            &[("identity_fields", "the typed identity field slot")],
            &["save", "discard"],
            &["saved", "discarded"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("profile", "react-jsx"),
                nextjs_adapter("profile", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::profile_lists_save_discard",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "register",
            UI_PATTERN_CATALOG_VERSION,
            "register",
            "Registration surface with typed fields, pending submit, failure and welcome state.",
            &[
                ("pending", "the typed pending submit surface"),
                ("failure", "the typed failure surface"),
                ("welcome", "the typed welcome surface"),
            ],
            &["submit"],
            &["registered"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("register", "react-jsx"),
                nextjs_adapter("register", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::register_lists_welcome_state",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "settings",
            UI_PATTERN_CATALOG_VERSION,
            "settings",
            "Settings surface with grouped preferences, explicit save and a typed reset action.",
            &[("grouped_preferences", "the typed grouped preferences slot")],
            &["save", "reset"],
            &["saved", "reset"],
            &["validated-form"],
            &["auth"],
            vec![
                react_adapter("settings", "react-jsx"),
                nextjs_adapter("settings", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::settings_lists_grouped_preferences",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "success-page",
            UI_PATTERN_CATALOG_VERSION,
            "success-page",
            "Full-page success surface with a confirmation message and a continue action.",
            &[("confirmation", "the typed confirmation message slot")],
            &["continue"],
            &["continued"],
            &[],
            &[],
            vec![
                react_adapter("success-page", "react-jsx"),
                nextjs_adapter("success-page", "nextjs-jsx"),
            ],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::success_page_lists_continue",
            UiPatternQuality::Verified,
            verified_evidence(),
        ),
        descriptor(
            "webhook-receiver",
            UI_PATTERN_CATALOG_VERSION,
            "form",
            "Legacy webhook-receiver surface kept only for migration.",
            &[("pending", "the typed pending submit surface")],
            &["submit"],
            &["submitted"],
            &["webhook-receiver"],
            &[],
            vec![nextjs_adapter("webhook-receiver", "nextjs-jsx")],
            DEFAULT_INSTALL_STRATEGY,
            "tests/ui_pattern_contract.rs::webhook_receiver_is_deprecated",
            UiPatternQuality::Deprecated,
            deprecated_evidence(),
        ),
    ];
    out.sort_by(|a, b| a.id.cmp(&b.id));
    for descriptor in &out {
        validate_descriptor(descriptor)
            .unwrap_or_else(|err| panic!("catalog entry failed validation: {err}"));
    }
    out
}
