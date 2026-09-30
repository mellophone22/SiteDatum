use serde::{Deserialize, Serialize};

pub const FREE_ACTIVE_PROJECT_LIMIT: usize = 3;
pub const PRO_DEVICE_LIMIT: usize = 2;
pub const OFFLINE_GRACE_SECONDS: i64 = 21 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    ProMonthly,
    ProAnnual,
}

impl Plan {
    pub fn is_pro(self) -> bool {
        matches!(self, Self::ProMonthly | Self::ProAnnual)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Canceled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementFreshness {
    Free,
    Verified,
    Grace,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommercialFeature {
    ManualWorkflows,
    LocalBackupRestore,
    CsvDataExport,
    ProjectTemplates,
    BulkOperations,
    SpreadsheetImport,
    SpreadsheetExport,
    ProfessionalReports,
    MetadataSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntitlementEvidence {
    pub plan: Plan,
    pub subscription_status: SubscriptionStatus,
    pub paid_through_utc: Option<i64>,
    pub last_verified_utc: Option<i64>,
    pub verification_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveEntitlement {
    pub plan: Plan,
    pub freshness: EntitlementFreshness,
}

impl EffectiveEntitlement {
    pub fn free() -> Self {
        Self {
            plan: Plan::Free,
            freshness: EntitlementFreshness::Free,
        }
    }

    pub fn is_pro(self) -> bool {
        self.plan.is_pro()
            && matches!(
                self.freshness,
                EntitlementFreshness::Verified | EntitlementFreshness::Grace
            )
    }

    pub fn active_project_limit(self) -> Option<usize> {
        (!self.is_pro()).then_some(FREE_ACTIVE_PROJECT_LIMIT)
    }

    pub fn can_activate_project(self, current_active_projects: usize) -> bool {
        self.active_project_limit()
            .is_none_or(|limit| current_active_projects < limit)
    }

    pub fn can_edit_existing_records(self) -> bool {
        true
    }

    pub fn can_use_feature(self, feature: CommercialFeature) -> bool {
        match feature {
            CommercialFeature::ManualWorkflows
            | CommercialFeature::LocalBackupRestore
            | CommercialFeature::CsvDataExport => true,
            CommercialFeature::MetadataSync => false,
            CommercialFeature::ProjectTemplates
            | CommercialFeature::BulkOperations
            | CommercialFeature::SpreadsheetImport
            | CommercialFeature::SpreadsheetExport
            | CommercialFeature::ProfessionalReports => self.is_pro(),
        }
    }

    pub fn device_limit(self) -> Option<usize> {
        self.is_pro().then_some(PRO_DEVICE_LIMIT)
    }
}

pub fn evaluate_entitlement(evidence: EntitlementEvidence, now_utc: i64) -> EffectiveEntitlement {
    if !evidence.plan.is_pro() {
        return EffectiveEntitlement::free();
    }

    let Some(paid_through_utc) = evidence.paid_through_utc else {
        return expired_entitlement();
    };
    if evidence.subscription_status == SubscriptionStatus::Expired || now_utc >= paid_through_utc {
        return expired_entitlement();
    }

    if evidence.verification_available {
        return EffectiveEntitlement {
            plan: evidence.plan,
            freshness: EntitlementFreshness::Verified,
        };
    }

    let Some(last_verified_utc) = evidence.last_verified_utc else {
        return expired_entitlement();
    };
    let Some(unverified_seconds) = now_utc.checked_sub(last_verified_utc) else {
        return expired_entitlement();
    };
    if unverified_seconds < 0 || unverified_seconds > OFFLINE_GRACE_SECONDS {
        return expired_entitlement();
    }

    EffectiveEntitlement {
        plan: evidence.plan,
        freshness: EntitlementFreshness::Grace,
    }
}

fn expired_entitlement() -> EffectiveEntitlement {
    EffectiveEntitlement {
        plan: Plan::Free,
        freshness: EntitlementFreshness::Expired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 2_000_000_000;

    fn evidence(plan: Plan) -> EntitlementEvidence {
        EntitlementEvidence {
            plan,
            subscription_status: SubscriptionStatus::Active,
            paid_through_utc: Some(NOW + 30 * 24 * 60 * 60),
            last_verified_utc: Some(NOW),
            verification_available: true,
        }
    }

    #[test]
    fn free_allows_three_active_projects_and_all_existing_edits() {
        let entitlement = evaluate_entitlement(evidence(Plan::Free), NOW);

        assert_eq!(
            entitlement.active_project_limit(),
            Some(FREE_ACTIVE_PROJECT_LIMIT)
        );
        assert!(entitlement.can_activate_project(0));
        assert!(entitlement.can_activate_project(2));
        assert!(!entitlement.can_activate_project(3));
        assert!(entitlement.can_edit_existing_records());
    }

    #[test]
    fn pro_has_unlimited_projects_and_two_devices() {
        for plan in [Plan::ProMonthly, Plan::ProAnnual] {
            let entitlement = evaluate_entitlement(evidence(plan), NOW);
            assert!(entitlement.is_pro());
            assert_eq!(entitlement.active_project_limit(), None);
            assert!(entitlement.can_activate_project(usize::MAX));
            assert_eq!(entitlement.device_limit(), Some(PRO_DEVICE_LIMIT));
        }
    }

    #[test]
    fn portability_is_free_and_productivity_features_require_pro() {
        let free = EffectiveEntitlement::free();
        let pro = evaluate_entitlement(evidence(Plan::ProMonthly), NOW);

        for feature in [
            CommercialFeature::ManualWorkflows,
            CommercialFeature::LocalBackupRestore,
            CommercialFeature::CsvDataExport,
        ] {
            assert!(free.can_use_feature(feature));
            assert!(pro.can_use_feature(feature));
        }
        for feature in [
            CommercialFeature::ProjectTemplates,
            CommercialFeature::BulkOperations,
            CommercialFeature::SpreadsheetImport,
            CommercialFeature::SpreadsheetExport,
            CommercialFeature::ProfessionalReports,
        ] {
            assert!(!free.can_use_feature(feature));
            assert!(pro.can_use_feature(feature));
        }
        assert!(!free.can_use_feature(CommercialFeature::MetadataSync));
        assert!(!pro.can_use_feature(CommercialFeature::MetadataSync));
    }

    #[test]
    fn unavailable_verification_uses_an_inclusive_twenty_one_day_grace() {
        let mut value = evidence(Plan::ProMonthly);
        value.verification_available = false;
        value.last_verified_utc = Some(NOW - OFFLINE_GRACE_SECONDS);

        let last_grace_instant = evaluate_entitlement(value, NOW);
        assert_eq!(last_grace_instant.freshness, EntitlementFreshness::Grace);
        assert!(last_grace_instant.is_pro());

        value.last_verified_utc = Some(NOW - OFFLINE_GRACE_SECONDS - 1);
        let expired = evaluate_entitlement(value, NOW);
        assert_eq!(expired.freshness, EntitlementFreshness::Expired);
        assert!(!expired.is_pro());
        assert!(expired.can_edit_existing_records());
    }

    #[test]
    fn known_expiration_overrides_grace_and_retains_existing_edits() {
        let mut value = evidence(Plan::ProAnnual);
        value.subscription_status = SubscriptionStatus::Expired;
        value.verification_available = false;

        let entitlement = evaluate_entitlement(value, NOW);
        assert_eq!(entitlement.freshness, EntitlementFreshness::Expired);
        assert_eq!(entitlement.plan, Plan::Free);
        assert!(entitlement.can_edit_existing_records());
    }

    #[test]
    fn cancellation_and_past_due_remain_pro_until_paid_through() {
        for status in [SubscriptionStatus::Canceled, SubscriptionStatus::PastDue] {
            let mut value = evidence(Plan::ProMonthly);
            value.subscription_status = status;
            value.paid_through_utc = Some(NOW + 1);
            assert!(evaluate_entitlement(value, NOW).is_pro());
            assert!(!evaluate_entitlement(value, NOW + 1).is_pro());
        }
    }

    #[test]
    fn missing_evidence_and_clock_rollback_fail_closed_without_hiding_data() {
        let mut value = evidence(Plan::ProMonthly);
        value.paid_through_utc = None;
        assert!(!evaluate_entitlement(value, NOW).is_pro());

        value.paid_through_utc = Some(NOW + 100);
        value.verification_available = false;
        value.last_verified_utc = Some(NOW + 1);
        let clock_rollback = evaluate_entitlement(value, NOW);
        assert!(!clock_rollback.is_pro());
        assert!(clock_rollback.can_edit_existing_records());
    }
}
