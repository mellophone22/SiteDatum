use crate::entitlement::{CommercialFeature, EffectiveEntitlement};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommercialAccess {
    Precommercial,
    // C3 will construct this from authenticated, tamper-resistant evidence.
    #[allow(dead_code)]
    Enforced(EffectiveEntitlement),
}

pub fn require_project_activation(
    access: CommercialAccess,
    current_active_projects: usize,
) -> AppResult<()> {
    if matches!(access, CommercialAccess::Precommercial)
        || matches!(access, CommercialAccess::Enforced(entitlement) if entitlement.can_activate_project(current_active_projects))
    {
        return Ok(());
    }

    Err(AppError::from_technical(
        "ACTIVE_PROJECT_LIMIT_REACHED",
        "The Free plan supports up to three active projects.",
        "Archive an active project before creating or restoring another, or activate Pro.",
        format!("active project count: {current_active_projects}"),
    ))
}

pub fn require_feature(access: CommercialAccess, feature: CommercialFeature) -> AppResult<()> {
    if matches!(access, CommercialAccess::Precommercial)
        || matches!(access, CommercialAccess::Enforced(entitlement) if entitlement.can_use_feature(feature))
    {
        return Ok(());
    }

    Err(AppError::from_technical(
        "PRO_FEATURE_REQUIRED",
        "This workflow requires SiteDatum Pro.",
        "Continue with the equivalent manual workflow or activate Pro.",
        format!("denied commercial feature: {feature:?}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlement::{EntitlementFreshness, Plan};

    fn pro() -> EffectiveEntitlement {
        EffectiveEntitlement {
            plan: Plan::ProMonthly,
            freshness: EntitlementFreshness::Verified,
        }
    }

    #[test]
    fn free_project_activation_is_allowed_only_below_the_limit() {
        assert!(require_project_activation(CommercialAccess::Precommercial, usize::MAX).is_ok());
        assert!(require_project_activation(
            CommercialAccess::Enforced(EffectiveEntitlement::free()),
            2
        )
        .is_ok());
        let error =
            require_project_activation(CommercialAccess::Enforced(EffectiveEntitlement::free()), 3)
                .unwrap_err();
        assert_eq!(error.code, "ACTIVE_PROJECT_LIMIT_REACHED");
        assert!(require_project_activation(CommercialAccess::Enforced(pro()), usize::MAX).is_ok());
    }

    #[test]
    fn free_portability_remains_available_while_pro_mutations_are_denied() {
        let free = EffectiveEntitlement::free();
        assert!(require_feature(
            CommercialAccess::Enforced(free),
            CommercialFeature::LocalBackupRestore
        )
        .is_ok());
        assert!(require_feature(
            CommercialAccess::Enforced(free),
            CommercialFeature::CsvDataExport
        )
        .is_ok());

        for feature in [
            CommercialFeature::ProjectTemplates,
            CommercialFeature::BulkOperations,
            CommercialFeature::SpreadsheetImport,
            CommercialFeature::SpreadsheetExport,
            CommercialFeature::ProfessionalReports,
        ] {
            assert!(require_feature(CommercialAccess::Precommercial, feature).is_ok());
            let error = require_feature(CommercialAccess::Enforced(free), feature).unwrap_err();
            assert_eq!(error.code, "PRO_FEATURE_REQUIRED");
            assert!(require_feature(CommercialAccess::Enforced(pro()), feature).is_ok());
        }
    }
}
