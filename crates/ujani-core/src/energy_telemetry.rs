//! Auditable energy observations. This module stores typed observations; it does not meter hardware.
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum MeasurementKind{DirectMeter,OperatingSystemEstimate,ModelledEstimate}
#[derive(Debug,Clone,PartialEq,Eq)] pub struct EnergyObservation{pub workload_id:String,pub device_id:String,pub kind:MeasurementKind,pub energy_mwh:u64,pub duration_ms:u64,pub completed_successfully:bool,pub output_quality_score_milli:Option<u16>,pub source:String,pub recorded_at_unix_ms:u64}
#[derive(Debug,Clone,PartialEq,Eq)] pub enum ObservationError{MissingIdentity,MissingProvenance,ZeroDuration,QualityOutOfRange,FailedWorkload}
pub fn validate_observation(o:&EnergyObservation)->Result<(),ObservationError>{
 if o.workload_id.trim().is_empty()||o.device_id.trim().is_empty(){return Err(ObservationError::MissingIdentity)}
 if o.source.trim().is_empty()||o.recorded_at_unix_ms==0{return Err(ObservationError::MissingProvenance)}
 if o.duration_ms==0{return Err(ObservationError::ZeroDuration)}
 if o.output_quality_score_milli.is_some_and(|v|v>1000){return Err(ObservationError::QualityOutOfRange)}
 if !o.completed_successfully{return Err(ObservationError::FailedWorkload)} Ok(())
}
#[derive(Debug,Clone,PartialEq,Eq)] pub struct DashboardTotals{pub direct_metered_mwh:u128,pub os_estimated_mwh:u128,pub modelled_estimated_mwh:u128,pub accepted_observations:usize}
pub fn aggregate_observations(items:&[EnergyObservation])->DashboardTotals{
 let mut t=DashboardTotals{direct_metered_mwh:0,os_estimated_mwh:0,modelled_estimated_mwh:0,accepted_observations:0};
 for o in items{if validate_observation(o).is_err(){continue} t.accepted_observations+=1;match o.kind{MeasurementKind::DirectMeter=>t.direct_metered_mwh+=u128::from(o.energy_mwh),MeasurementKind::OperatingSystemEstimate=>t.os_estimated_mwh+=u128::from(o.energy_mwh),MeasurementKind::ModelledEstimate=>t.modelled_estimated_mwh+=u128::from(o.energy_mwh)}} t
}
#[cfg(test)] mod tests{use super::*;fn obs(kind:MeasurementKind)->EnergyObservation{EnergyObservation{workload_id:"job-1".into(),device_id:"pc-1".into(),kind,energy_mwh:500,duration_ms:1000,completed_successfully:true,output_quality_score_milli:Some(950),source:"meter-v1".into(),recorded_at_unix_ms:1}}
#[test]fn separates_measured_and_estimated(){let t=aggregate_observations(&[obs(MeasurementKind::DirectMeter),obs(MeasurementKind::OperatingSystemEstimate),obs(MeasurementKind::ModelledEstimate)]);assert_eq!(t.direct_metered_mwh,500);assert_eq!(t.os_estimated_mwh,500);assert_eq!(t.modelled_estimated_mwh,500);assert_eq!(t.accepted_observations,3)}
#[test]fn rejects_failed_work(){let mut o=obs(MeasurementKind::DirectMeter);o.completed_successfully=false;assert_eq!(validate_observation(&o),Err(ObservationError::FailedWorkload))}
#[test]fn rejects_missing_provenance(){let mut o=obs(MeasurementKind::DirectMeter);o.source.clear();assert_eq!(validate_observation(&o),Err(ObservationError::MissingProvenance))}
#[test]fn rejects_invalid_quality(){let mut o=obs(MeasurementKind::DirectMeter);o.output_quality_score_milli=Some(1001);assert_eq!(validate_observation(&o),Err(ObservationError::QualityOutOfRange))}
}