//! Parameters for meta SDF nodes.

use anyhow::{Result, bail};
use impact_alloc::{AVec, arena::ArenaPool, avec};
use impact_containers::FixedQueue;
use impact_math::{
    angle::{degrees_to_radians, radians_to_degrees},
    power_law::sample_power_law,
};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use std::{f32::consts::PI, mem};
use tinyvec::TinyVec;

pub type ParamRng = Pcg64Mcg;

pub type ParamIdx = u16;

#[derive(Clone, Debug)]
pub enum ParamSpecRef<'a> {
    Discrete(&'a DiscreteParamSpec),
    Continuous(&'a ContParamSpec),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum DiscreteParamSpec {
    Constant(DiscreteValueSource),
    Uniform {
        min: DiscreteValueSource,
        max: DiscreteValueSource,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum ContParamSpec {
    Constant(ContValueSource),
    Uniform {
        min: ContValueSource,
        max: ContValueSource,
    },
    UniformCosAngle {
        min_angle: ContValueSource,
        max_angle: ContValueSource,
    },
    PowerLaw {
        min: ContValueSource,
        max: ContValueSource,
        exponent: ContValueSource,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum DiscreteValueSource {
    Fixed(u32),
    FromParam {
        idx: ParamIdx,
        mapping: ParamValueMapping,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum ContValueSource {
    Fixed(f32),
    FromParam {
        idx: ParamIdx,
        mapping: ParamValueMapping,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug)]
pub enum ParamValueMapping {
    Linear { offset: f32, scale: f32 },
}

type ParamIndicesForDeps = TinyVec<[ParamIdx; 4]>;

impl<'a> ParamSpecRef<'a> {
    fn param_dependencies(&self) -> ParamIndicesForDeps {
        match self {
            Self::Discrete(spec) => spec.param_dependencies(),
            Self::Continuous(spec) => spec.param_dependencies(),
        }
    }

    fn sample(&self, param_values: &[f32], rng: &mut ParamRng) -> f32 {
        match self {
            Self::Discrete(spec) => spec.sample(param_values, rng) as f32,
            Self::Continuous(spec) => spec.sample(param_values, rng),
        }
    }
}

impl DiscreteParamSpec {
    pub fn as_spec<'a>(&'a self) -> ParamSpecRef<'a> {
        ParamSpecRef::Discrete(self)
    }

    fn param_dependencies(&self) -> ParamIndicesForDeps {
        let mut deps = ParamIndicesForDeps::new();
        match self {
            Self::Constant(v) => v.add_param_dependency(&mut deps),
            Self::Uniform { min, max } => {
                min.add_param_dependency(&mut deps);
                max.add_param_dependency(&mut deps);
            }
        }
        deps
    }

    fn sample(&self, param_values: &[f32], rng: &mut ParamRng) -> u32 {
        match self {
            Self::Constant(constant) => constant.eval(param_values),
            Self::Uniform { min, max } => {
                let min_value = min.eval(param_values);
                let max_value = max.eval(param_values).max(min_value);
                rng.random_range(min_value..=max_value)
            }
        }
    }
}

impl ContParamSpec {
    pub fn as_spec<'a>(&'a self) -> ParamSpecRef<'a> {
        ParamSpecRef::Continuous(self)
    }

    fn param_dependencies(&self) -> ParamIndicesForDeps {
        let mut deps = ParamIndicesForDeps::new();
        match self {
            Self::Constant(v) => v.add_param_dependency(&mut deps),
            Self::Uniform { min, max }
            | Self::UniformCosAngle {
                min_angle: min,
                max_angle: max,
            } => {
                min.add_param_dependency(&mut deps);
                max.add_param_dependency(&mut deps);
            }
            Self::PowerLaw { min, max, exponent } => {
                min.add_param_dependency(&mut deps);
                max.add_param_dependency(&mut deps);
                exponent.add_param_dependency(&mut deps);
            }
        }
        deps
    }

    fn sample(&self, param_values: &[f32], rng: &mut ParamRng) -> f32 {
        match self {
            Self::Constant(constant) => constant.eval(param_values),
            Self::Uniform { min, max } => {
                let min_value = min.eval(param_values);
                let max_value = max.eval(param_values).max(min_value);
                rng.random_range(min_value..=max_value)
            }
            Self::UniformCosAngle {
                min_angle,
                max_angle,
            } => {
                let mut min_angle = degrees_to_radians(min_angle.eval(param_values));
                let mut max_angle = degrees_to_radians(max_angle.eval(param_values));

                min_angle = min_angle.clamp(0.0, PI);
                max_angle = max_angle.clamp(min_angle, PI);

                let min_cos = f32::cos(max_angle);
                let max_cos = f32::cos(min_angle);

                let cos_angle = rng.random_range(min_cos..=max_cos);

                radians_to_degrees(f32::acos(cos_angle))
            }
            Self::PowerLaw { min, max, exponent } => {
                let min_value = min.eval(param_values);
                let max_value = max.eval(param_values).max(min_value);
                let exponent = exponent.eval(param_values);
                sample_power_law(min_value, max_value, exponent, rng.random())
            }
        }
    }
}

impl DiscreteValueSource {
    fn add_param_dependency(&self, deps: &mut ParamIndicesForDeps) {
        if let Self::FromParam { idx, .. } = self {
            deps.push(*idx);
        }
    }

    fn eval(&self, param_values: &[f32]) -> u32 {
        match self {
            Self::Fixed(value) => *value,
            Self::FromParam { idx, mapping } => {
                let unmapped_value = param_values[*idx as usize];
                let float_value = mapping.apply(unmapped_value);
                float_value.round().max(0.0) as u32
            }
        }
    }
}

impl ContValueSource {
    fn add_param_dependency(&self, deps: &mut ParamIndicesForDeps) {
        if let Self::FromParam { idx, .. } = self {
            deps.push(*idx);
        }
    }

    fn eval(&self, param_values: &[f32]) -> f32 {
        match self {
            Self::Fixed(value) => *value,
            Self::FromParam { idx, mapping } => {
                let unmapped_value = param_values[*idx as usize];
                mapping.apply(unmapped_value)
            }
        }
    }
}

impl ParamValueMapping {
    fn apply(&self, value: f32) -> f32 {
        match *self {
            Self::Linear { offset, scale } => offset + scale * value,
        }
    }
}

pub fn create_param_rng(seed: u64) -> ParamRng {
    ParamRng::seed_from_u64(seed)
}

pub fn evaluate_params_for_node<const N: usize>(
    param_specs: &[ParamSpecRef<'_>; N],
    evaluated_params: &mut [f32; N],
    rng: &mut ParamRng,
) -> Result<()> {
    let mut eval_order = [0; N];
    compute_param_eval_order(param_specs, &mut eval_order)?;

    for param_idx in eval_order {
        let value = param_specs[param_idx as usize].sample(evaluated_params, rng);
        evaluated_params[param_idx as usize] = value;
    }

    Ok(())
}

fn compute_param_eval_order<const N: usize>(
    specs: &[ParamSpecRef<'_>; N],
    eval_order: &mut [ParamIdx; N],
) -> Result<()> {
    if specs.is_empty() {
        return Ok(());
    }

    let n_params = specs.len();

    // Estimate capacity based on parameter count for dependency tracking
    let capacity = n_params * (mem::size_of::<usize>() * 2 + 64); // dep_counts + queue + overhead
    let arena = ArenaPool::get_arena_for_capacity(capacity);

    let mut dep_counts = avec![in &arena; 0; n_params];

    let mut reverse_deps = AVec::new_in(&arena);
    reverse_deps.resize_with(n_params, ParamIndicesForDeps::new);

    let mut queue = FixedQueue::with_capacity_in(n_params, &arena);

    // Count dependencies for each parameter and store the indices of the
    // parameters that depend on them
    for (param_idx, spec) in specs.iter().enumerate() {
        for dep_idx in spec.param_dependencies() {
            if dep_idx >= n_params as ParamIdx {
                bail!(
                    "Parameter {} depends on out-of-range parameter {}",
                    param_idx,
                    dep_idx
                );
            }

            dep_counts[param_idx] += 1;
            reverse_deps[dep_idx as usize].push(param_idx as ParamIdx);
        }
    }

    // Queue each parameter with no dependencies
    for param_idx in 0..n_params {
        if dep_counts[param_idx] == 0 {
            queue.push_back(param_idx as ParamIdx);
        }
    }

    // Traverse in topological order to determine evaluation order
    let mut eval_counter = 0;
    while let Some(param_idx) = queue.pop_front() {
        eval_order[eval_counter] = param_idx;
        eval_counter += 1;

        for &rev_dep_idx in &reverse_deps[param_idx as usize] {
            // Decrement remaining dependecy count and enqueue when ready
            dep_counts[rev_dep_idx as usize] -= 1;
            if dep_counts[rev_dep_idx as usize] == 0 {
                queue.push_back(rev_dep_idx);
            }
        }
    }

    if eval_counter != n_params {
        bail!("Cycle in parameter dependencies");
    }

    Ok(())
}

#[macro_export]
macro_rules! define_meta_node_params {
    (
        $node:ty,
        struct $params:ident {
            $( $field:ident : $typ:ty ),+ $(,)?
        }
    ) => {
        #[derive(Clone, Copy, Debug)]
        struct $params {
            $( $field: $typ, )+
        }

        impl $node {
            fn sample_params(
                &self,
                rng: &mut $crate::generation::sdf::meta::params::ParamRng,
            ) -> ::anyhow::Result<$params>
            {
                const N: usize = define_meta_node_params!(@count $( $field )+ );
                let specs: [$crate::generation::sdf::meta::params::ParamSpecRef<'_>; N] = [
                    $( self.$field.as_spec(), )+
                ];
                let mut values = [0.0; N];
                $crate::generation::sdf::meta::params::evaluate_params_for_node(&specs, &mut values, rng)?;

                let mut _idx = 0;
                Ok($params {
                    $(
                        $field: { let v = values[_idx]; _idx += 1; v as $typ },
                    )+
                })
            }
        }
    };

    // Helper to count fields
    (@count $($field:ident)+) => {
        <[()]>::len(&[ $( define_meta_node_params!(@unit $field) ),+ ])
    };
    (@unit $field:ident) => { () };
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXED: DiscreteParamSpec = DiscreteParamSpec::Constant(DiscreteValueSource::Fixed(0));

    fn fixed() -> ParamSpecRef<'static> {
        FIXED.as_spec()
    }

    const fn from_param(idx: ParamIdx) -> DiscreteParamSpec {
        DiscreteParamSpec::Constant(DiscreteValueSource::FromParam {
            idx,
            mapping: ParamValueMapping::Linear {
                offset: 0.0,
                scale: 1.0,
            },
        })
    }

    #[test]
    fn compute_param_eval_order_with_empty_specs_succeeds() {
        let specs: [ParamSpecRef<'_>; 0] = [];
        let mut eval_order: [ParamIdx; 0] = [];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_ok());
    }

    #[test]
    fn compute_param_eval_order_with_single_independent_param_works() {
        let specs = [fixed()];
        let mut eval_order = [0];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_ok());
        assert_eq!(eval_order, [0]);
    }

    #[test]
    fn compute_param_eval_order_with_multiple_independent_params_works() {
        let specs = [fixed(), fixed(), fixed()];
        let mut eval_order = [0; 3];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_ok());

        // All parameters should be included exactly once
        let mut sorted_order = eval_order;
        sorted_order.sort();
        assert_eq!(sorted_order, [0, 1, 2]);
    }

    #[test]
    fn compute_param_eval_order_with_linear_dependency_chain_works() {
        // Param 0: independent
        // Param 1: depends on param 0
        // Param 2: depends on param 1
        let spec1 = from_param(0);
        let spec2 = from_param(1);
        let specs = [fixed(), spec1.as_spec(), spec2.as_spec()];
        let mut eval_order = [0; 3];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_ok());
        assert_eq!(eval_order, [0, 1, 2]);
    }

    #[test]
    fn compute_param_eval_order_with_reverse_linear_dependency_chain_works() {
        // Param 0: depends on param 1
        // Param 1: depends on param 2
        // Param 2: independent
        let spec0 = from_param(1);
        let spec1 = from_param(2);
        let specs = [spec0.as_spec(), spec1.as_spec(), fixed()];
        let mut eval_order = [0; 3];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_ok());
        assert_eq!(eval_order, [2, 1, 0]);
    }

    #[test]
    fn compute_param_eval_order_with_long_dependency_chain_works() {
        // Param 0: independent
        // Param 1: depends on param 0
        // Param 2: depends on param 1
        // Param 3: depends on param 2
        let spec1 = from_param(0);
        let spec2 = from_param(1);
        let spec3 = from_param(2);
        let specs = [fixed(), spec1.as_spec(), spec2.as_spec(), spec3.as_spec()];
        let mut eval_order = [0; 4];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_ok());
        assert_eq!(eval_order, [0, 1, 2, 3]);
    }

    #[test]
    fn compute_param_eval_order_with_circular_dependency_fails() {
        // Param 0: depends on param 1
        // Param 1: depends on param 0
        let spec0 = from_param(1);
        let spec1 = from_param(0);
        let specs = [spec0.as_spec(), spec1.as_spec()];
        let mut eval_order = [0, 0];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle"));
    }

    #[test]
    fn compute_param_eval_order_with_self_dependency_fails() {
        // Param 0: depends on itself
        let spec = from_param(0);
        let specs = [spec.as_spec()];
        let mut eval_order = [0];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle"));
    }

    #[test]
    fn compute_param_eval_order_with_three_way_cycle_fails() {
        // Param 0: depends on param 2
        // Param 1: depends on param 0
        // Param 2: depends on param 1
        let spec0 = from_param(2);
        let spec1 = from_param(0);
        let spec2 = from_param(1);
        let specs = [spec0.as_spec(), spec1.as_spec(), spec2.as_spec()];
        let mut eval_order = [0, 0, 0];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle"));
    }

    #[test]
    fn compute_param_eval_order_with_out_of_range_dependency_fails() {
        // Param 0: depends on param 5 (out of range)
        let spec = from_param(5);
        let specs = [spec.as_spec()];
        let mut eval_order = [0];

        let result = compute_param_eval_order(&specs, &mut eval_order);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("range"));
    }
}
