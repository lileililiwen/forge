//! The runtime-kind vocabulary's own suite.

#[cfg(test)]
pub(super) mod tests {
    use crate::core::ForgeError;
    use crate::runtime::{
        parse_runtime_kind, HostModel, RuntimeKind, RUNTIME_KINDS, RUNTIME_KIND_NAMES,
        SHARED_HOST_REASON,
    };

    /// Every kind the vocabulary declares, with its host model.
    const KINDS: &[(RuntimeKind, HostModel)] = &[
        (RuntimeKind::Web, HostModel::OwnProcess),
        (RuntimeKind::Worker, HostModel::OwnProcess),
        (RuntimeKind::Job, HostModel::OwnProcess),
        (RuntimeKind::Library, HostModel::SharedHost),
    ];

    #[test]
    fn every_kind_declares_its_host_model() {
        for (kind, host_model) in KINDS {
            assert_eq!(
                kind.host_model(),
                *host_model,
                "{} reports the wrong host model",
                kind.as_str()
            );
        }
    }

    #[test]
    fn only_the_library_kind_is_shared_host() {
        let shared: Vec<&str> = KINDS
            .iter()
            .filter(|(kind, _)| kind.host_model() == HostModel::SharedHost)
            .map(|(kind, _)| kind.as_str())
            .collect();
        assert_eq!(shared, vec!["library"]);
    }

    #[test]
    fn the_shared_host_model_is_the_module_host() {
        assert_eq!(HostModel::SharedHost.as_str(), "shared_host");
        assert_eq!(HostModel::OwnProcess.as_str(), "own_process");
        assert_eq!(
            HostModel::SharedHost.to_string(),
            "shared_host",
            "the display impl must agree with as_str"
        );
        assert_eq!(HostModel::OwnProcess.to_string(), "own_process");
    }

    #[test]
    fn the_parser_accepts_the_four_values_trimmed_and_in_any_case() {
        for (kind, _) in KINDS {
            let spelling = kind.as_str();
            assert_eq!(RuntimeKind::parse(spelling), Some(*kind));
            assert_eq!(
                RuntimeKind::parse(&spelling.to_ascii_uppercase()),
                Some(*kind)
            );
            assert_eq!(RuntimeKind::parse(&format!("  {spelling}  ")), Some(*kind));
            assert_eq!(
                parse_runtime_kind(&format!(" {spelling} ")).ok(),
                Some(*kind),
                "the shared parser accepts the trimmed spelling"
            );
        }
    }

    #[test]
    fn the_parser_refuses_everything_else_by_name() {
        for refused in ["", " ", "svc", "database", "web-worker", "libs"] {
            assert_eq!(
                RuntimeKind::parse(refused),
                None,
                "`{refused}` is not a kind"
            );
            let err = parse_runtime_kind(refused).expect_err("an unknown kind is refused");
            assert!(
                matches!(err, ForgeError::BuildInvalid { .. }),
                "the refusal must use the shared error type, got {err:?}"
            );
            let reason = err.to_string();
            assert!(
                reason.contains(refused.trim()) || refused.trim().is_empty(),
                "the refusal must name the refused value, got {reason}"
            );
            for accepted in RUNTIME_KIND_NAMES {
                assert!(
                    reason.contains(accepted),
                    "the refusal must name `{accepted}`, got {reason}"
                );
            }
        }
    }

    #[test]
    fn the_display_impl_and_as_str_agree() {
        for (kind, _) in KINDS {
            assert_eq!(kind.to_string(), kind.as_str());
        }
    }

    #[test]
    fn the_table_and_the_enum_agree() {
        assert_eq!(
            RUNTIME_KINDS.len(),
            KINDS.len(),
            "the table must carry one row per kind"
        );
        for (row, (kind, host_model)) in RUNTIME_KINDS.iter().zip(KINDS.iter()) {
            assert_eq!(row.kind, *kind, "the table row names the wrong kind");
            assert_eq!(
                row.host_model,
                *host_model,
                "the table row for `{}` names the wrong host model",
                row.kind.as_str()
            );
            assert_eq!(
                row.summary,
                kind.summary(),
                "the table row for `{}` names the wrong summary",
                row.kind.as_str()
            );
        }
        for row in RUNTIME_KINDS.iter() {
            assert!(KINDS.iter().any(|(kind, _)| *kind == row.kind));
        }
    }

    #[test]
    fn the_canonical_spellings_match_the_table() {
        let names: Vec<&str> = RUNTIME_KINDS.iter().map(|row| row.kind.as_str()).collect();
        assert_eq!(names, RUNTIME_KIND_NAMES.to_vec());
    }

    #[test]
    fn the_kinds_serialize_with_the_contract_spellings() {
        for (kind, _) in KINDS {
            let encoded = serde_json::to_string(kind).expect("a kind encodes");
            assert_eq!(encoded, format!("\"{}\"", kind.as_str()));
            let decoded: RuntimeKind = serde_json::from_str(&encoded).expect("a kind decodes");
            assert_eq!(decoded, *kind);
        }
        for (kind, _) in KINDS {
            let encoded = serde_json::to_string(&kind.host_model()).expect("a host model encodes");
            assert_eq!(encoded, format!("\"{}\"", kind.host_model().as_str()));
            let decoded: HostModel = serde_json::from_str(&encoded).expect("a host model decodes");
            assert_eq!(decoded, kind.host_model());
        }
    }

    #[test]
    fn only_the_shared_host_model_holds_an_artifact_back() {
        assert_eq!(HostModel::OwnProcess.hold_back_reason(), None);
        assert_eq!(
            HostModel::SharedHost.hold_back_reason(),
            Some(SHARED_HOST_REASON)
        );
        assert!(
            SHARED_HOST_REASON.contains("module"),
            "the reason must say the artifact is a module, got {SHARED_HOST_REASON}"
        );
        assert!(
            SHARED_HOST_REASON.contains("adopts"),
            "the reason must say who owns the lifecycle, got {SHARED_HOST_REASON}"
        );
    }
}
