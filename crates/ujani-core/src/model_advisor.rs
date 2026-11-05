//! Conservative, explainable local model fit estimates; never guarantees fit.
use serde::{Deserialize, Serialize};
const MIB:u64=1024*1024;
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct InstalledModelSize { pub name:String, pub size_bytes:u64 }
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct ModelFitEstimate { pub name:String, pub size_bytes:u64, pub estimated_working_memory_mb:Option<u64>, pub estimated_to_fit:bool, pub reason:String }
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
pub struct ModelRecommendation { pub recommended_model:Option<String>, pub available_memory_mb:u64, pub usable_memory_budget_mb:u64, pub candidates:Vec<ModelFitEstimate>, pub explanation:String }
/// Uses 2× disk size + 512 MiB and reserves 25% of current available RAM.
pub fn recommend_local_model(models:&[InstalledModelSize],available_memory_mb:u64)->ModelRecommendation {
 let budget=available_memory_mb.saturating_mul(3)/4;
 let mut candidates:Vec<ModelFitEstimate>=models.iter().map(|m|{
  let estimate=if m.size_bytes==0 {None} else {Some((m.size_bytes.saturating_add(MIB-1)/MIB).saturating_mul(2).saturating_add(512))};
  let fits=estimate.is_some_and(|n|available_memory_mb>0&&n<=budget);
  let reason=match estimate {None=>"Model size unknown; fit cannot be estimated.".into(),Some(_) if available_memory_mb==0=>"Available RAM unknown; no recommendation.".into(),Some(n) if fits=>format!("Rough estimate {n} MiB fits advisory budget {budget} MiB."),Some(n)=>format!("Rough estimate {n} MiB exceeds advisory budget {budget} MiB.")};
  ModelFitEstimate{name:m.name.clone(),size_bytes:m.size_bytes,estimated_working_memory_mb:estimate,estimated_to_fit:fits,reason}
 }).collect();
 candidates.sort_by(|a,b|match(a.estimated_to_fit,b.estimated_to_fit){(true,false)=>std::cmp::Ordering::Less,(false,true)=>std::cmp::Ordering::Greater,_=>a.estimated_working_memory_mb.cmp(&b.estimated_working_memory_mb).then_with(||a.name.cmp(&b.name))});
 let recommended_model=candidates.iter().find(|c|c.estimated_to_fit).map(|c|c.name.clone());
 ModelRecommendation{recommended_model,available_memory_mb,usable_memory_budget_mb:budget,candidates,explanation:"Advisory only: model disk size × 2 + 512 MiB, with 25% RAM headroom. Quantisation, context length, runtime and accelerator offload change real usage. No automatic model switching.".into()}
}
#[cfg(test)] mod tests {use super::*;fn m(n:&str,mb:u64)->InstalledModelSize{InstalledModelSize{name:n.into(),size_bytes:mb*MIB}}
#[test]fn recommends_smallest_fit(){let r=recommend_local_model(&[m("large",3000),m("small",500),m("medium",1000)],6000);assert_eq!(r.recommended_model.as_deref(),Some("small"));}
#[test]fn unknown_ram_never_recommends(){assert!(recommend_local_model(&[m("tiny",1)],0).recommended_model.is_none());}
#[test]fn unknown_size_never_recommends(){let r=recommend_local_model(&[m("unknown",0)],32000);assert!(r.recommended_model.is_none());assert!(r.candidates[0].estimated_working_memory_mb.is_none());}
#[test]fn oversized_candidate_is_not_recommended(){assert!(recommend_local_model(&[m("large",8000)],8000).recommended_model.is_none());}}
