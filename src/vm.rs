use bellman::{gadgets::Assignment, groth16, Circuit, ConstraintSystem, SynthesisError};
use bls12_381::Bls12;
use bls12_381::Scalar;
use ff::{Field, PrimeField};
use rand::rngs::OsRng;
use std::ops::{AddAssign, MulAssign, SubAssign};
use std::time::Instant;

use crate::error::Result;

pub struct ZKVirtualMachine {
    pub constants: Vec<Scalar>,
    pub alloc: Vec<(AllocType, VariableIndex)>,
    pub ops: Vec<CryptoOperation>,
    pub constraints: Vec<ConstraintInstruction>,

    pub aux: Vec<Scalar>,

    pub params: Option<groth16::Parameters<Bls12>>,
    pub verifying_key: Option<groth16::PreparedVerifyingKey<Bls12>>,
}

pub type VariableIndex = usize;

pub enum VariableRef {
    Aux(VariableIndex),
    Local(VariableIndex),
}

pub enum CryptoOperation {
    Set(VariableRef, VariableRef),
    Mul(VariableRef, VariableRef),
    Add(VariableRef, VariableRef),
    Sub(VariableRef, VariableRef),
    Load(VariableRef, VariableIndex),
    Divide(VariableRef, VariableRef),
    Double(VariableRef),
    Square(VariableRef),
    Invert(VariableRef),
    UnpackBits(VariableRef, VariableRef, VariableRef),
    Local,
    Debug(String, VariableRef),
    DumpAlloc,
    DumpLocal,
}

#[derive(Clone)]
pub enum AllocType {
    Private,
    Public,
}

#[derive(Debug, Clone)]
pub enum ConstraintInstruction {
    Lc0Add(VariableIndex),
    Lc1Add(VariableIndex),
    Lc2Add(VariableIndex),
    Lc0Sub(VariableIndex),
    Lc1Sub(VariableIndex),
    Lc2Sub(VariableIndex),
    Lc0AddOne,
    Lc1AddOne,
    Lc2AddOne,
    Lc0SubOne,
    Lc1SubOne,
    Lc2SubOne,
    Lc0AddCoeff(VariableIndex, VariableIndex),
    Lc1AddCoeff(VariableIndex, VariableIndex),
    Lc2AddCoeff(VariableIndex, VariableIndex),
    Lc0AddConstant(VariableIndex),
    Lc1AddConstant(VariableIndex),
    Lc2AddConstant(VariableIndex),
    Enforce,
    LcCoeffReset,
    LcCoeffDouble,
}

#[derive(Debug)]
pub enum ZKVMError {
    DivisionByZero,
    MalformedRange,
}

impl ZKVirtualMachine {
    pub fn initialize(
        &mut self,
        params: &Vec<(VariableIndex, Scalar)>,
    ) -> std::result::Result<(), ZKVMError> {
        // Resize array
        self.aux = vec![Scalar::zero(); self.alloc.len()];

        // Copy over the parameters
        for (index, value) in params {
            //println!("Setting {} to {:?}", index, value);
            self.aux[*index] = *value;
        }

        let mut local_stack: Vec<Scalar> = Vec::new();

        for op in &self.ops {
            match op {
                CryptoOperation::Set(self_, other) => {
                    let other = match other {
                        VariableRef::Aux(index) => self.aux[*index].clone(),
                        VariableRef::Local(index) => local_stack[*index].clone(),
                    };
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    *self_ = other;
                }
                CryptoOperation::Mul(self_, other) => {
                    let other = match other {
                        VariableRef::Aux(index) => self.aux[*index].clone(),
                        VariableRef::Local(index) => local_stack[*index].clone(),
                    };
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    self_.mul_assign(other);
                }
                CryptoOperation::Add(self_, other) => {
                    let other = match other {
                        VariableRef::Aux(index) => self.aux[*index].clone(),
                        VariableRef::Local(index) => local_stack[*index].clone(),
                    };
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    self_.add_assign(other);
                }
                CryptoOperation::Sub(self_, other) => {
                    let other = match other {
                        VariableRef::Aux(index) => self.aux[*index].clone(),
                        VariableRef::Local(index) => local_stack[*index].clone(),
                    };
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    self_.sub_assign(other);
                }
                CryptoOperation::Load(self_, const_index) => {
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    *self_ = self.constants[*const_index];
                }
                CryptoOperation::Divide(self_, other) => {
                    let other = match other {
                        VariableRef::Aux(index) => self.aux[*index].clone(),
                        VariableRef::Local(index) => local_stack[*index].clone(),
                    };
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    let ret = other.invert().map(|other| *self_ * other);
                    if bool::from(ret.is_some()) {
                        *self_ = ret.unwrap();
                    } else {
                        return Err(ZKVMError::DivisionByZero);
                    }
                }
                CryptoOperation::Double(self_) => {
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    *self_ = self_.double();
                }
                CryptoOperation::Square(self_) => {
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    *self_ = self_.square();
                }
                CryptoOperation::Invert(self_) => {
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    if self_.is_zero() {
                        return Err(ZKVMError::DivisionByZero);
                    } else {
                        *self_ = self_.invert().unwrap();
                    }
                }
                CryptoOperation::UnpackBits(value, start, end) => {
                    let value = match value {
                        VariableRef::Aux(index) => self.aux[*index].clone(),
                        VariableRef::Local(index) => local_stack[*index].clone(),
                    };
                    let (self_, start_index, end_index) = match start {
                        VariableRef::Aux(start_index) => match end {
                            VariableRef::Aux(end_index) => (&mut self.aux, start_index, end_index),
                            VariableRef::Local(_) => {
                                return Err(ZKVMError::MalformedRange);
                            }
                        },
                        VariableRef::Local(start_index) => match end {
                            VariableRef::Aux(_) => {
                                return Err(ZKVMError::MalformedRange);
                            }
                            VariableRef::Local(end_index) => {
                                (&mut local_stack, start_index, end_index)
                            }
                        },
                    };
                    if start_index > end_index {
                        return Err(ZKVMError::MalformedRange);
                    }
                    if (end_index + 1) - start_index != 256 {
                        return Err(ZKVMError::MalformedRange);
                    }
                    if *end_index >= self_.len() {
                        return Err(ZKVMError::MalformedRange);
                    }

                    for (i, bit) in value.to_le_bits().into_iter().cloned().enumerate() {
                        match bit {
                            true => self_[start_index + i] = Scalar::one(),
                            false => self_[start_index + i] = Scalar::zero(),
                        }
                    }
                }
                CryptoOperation::Local => {
                    local_stack.push(Scalar::zero());
                }
                CryptoOperation::Debug(debug_str, self_) => {
                    let self_ = match self_ {
                        VariableRef::Aux(index) => &mut self.aux[*index],
                        VariableRef::Local(index) => &mut local_stack[*index],
                    };
                    println!("{}", debug_str);
                    println!("value = {:?}", self_);
                }
                CryptoOperation::DumpAlloc => {
                    println!("-------------------");
                    println!("alloc");
                    println!("-------------------");
                    for (i, value) in self.aux.iter().enumerate() {
                        println!("{}: {:?}", i, value);
                    }
                    println!("-------------------");
                }
                CryptoOperation::DumpLocal => {
                    println!("-------------------");
                    println!("local");
                    println!("-------------------");
                    for (i, value) in local_stack.iter().enumerate() {
                        println!("{}: {:?}", i, value);
                    }
                    println!("-------------------");
                }
            }
        }

        Ok(())
    }

    pub fn public(&self) -> Vec<(VariableIndex, Scalar)> {
        let mut publics = Vec::new();
        for (alloc_type, index) in &self.alloc {
            match alloc_type {
                AllocType::Private => {}
                AllocType::Public => {
                    let scalar = self.aux[*index].clone();
                    publics.push((*index, scalar));
                }
            }
        }
        publics
    }

    pub fn setup(&mut self) -> Result<()> {
        let start = Instant::now();
        // Create parameters for our circuit. In a production deployment these would
        // be generated securely using a multiparty computation.
        self.params = Some({
            let circuit = ZKVMCircuit {
                aux: vec![None; self.aux.len()],
                alloc: self.alloc.clone(),
                constraints: self.constraints.clone(),
                constants: self.constants.clone(),
            };
            groth16::generate_random_parameters::<Bls12, _, _>(circuit, &mut OsRng)?
        });

        println!("Setup: [{:?}]", start.elapsed());

        self.verifying_key = Some(groth16::prepare_verifying_key(
            &self.params.as_ref().unwrap().vk,
        ));
        Ok(())
    }

    pub fn prove(&self) -> groth16::Proof<Bls12> {
        let aux = self.aux.iter().map(|scalar| Some(scalar.clone())).collect();
        // Create an instance of our circuit (with the preimage as a witness).
        let circuit = ZKVMCircuit {
            aux,
            alloc: self.alloc.clone(),
            constraints: self.constraints.clone(),
            constants: self.constants.clone(),
        };

        let start = Instant::now();
        // Create a Groth16 proof with our parameters.
        let proof =
            groth16::create_random_proof(circuit, self.params.as_ref().unwrap(), &mut OsRng)
                .unwrap();
        println!("Prove: [{:?}]", start.elapsed());
        proof
    }

    pub fn verify(&self, proof: &groth16::Proof<Bls12>, public_values: &Vec<Scalar>) -> bool {
        let start = Instant::now();
        let is_passed =
            groth16::verify_proof(self.verifying_key.as_ref().unwrap(), proof, public_values)
                .is_ok();
        println!("Verify: [{:?}]", start.elapsed());
        is_passed
    }
}

pub struct ZKVMCircuit {
    aux: Vec<Option<bls12_381::Scalar>>,
    alloc: Vec<(AllocType, VariableIndex)>,
    constraints: Vec<ConstraintInstruction>,
    constants: Vec<Scalar>,
}

impl Circuit<bls12_381::Scalar> for ZKVMCircuit {
    fn synthesize<CS: ConstraintSystem<bls12_381::Scalar>>(
        self,
        cs: &mut CS,
    ) -> std::result::Result<(), SynthesisError> {
        let mut variables = Vec::new();

        for (alloc_type, index) in &self.alloc {
            match alloc_type {
                AllocType::Private => {
                    let var = cs.alloc(|| "private alloc", || Ok(*self.aux[*index].get()?))?;
                    variables.push(var);
                }
                AllocType::Public => {
                    let var = cs.alloc_input(|| "public alloc", || Ok(*self.aux[*index].get()?))?;
                    variables.push(var);
                }
            }
        }

        let mut coeff = bls12_381::Scalar::one();
        let mut lc0 = bellman::LinearCombination::<Scalar>::zero();
        let mut lc1 = bellman::LinearCombination::<Scalar>::zero();
        let mut lc2 = bellman::LinearCombination::<Scalar>::zero();

        for constraint in self.constraints {
            match constraint {
                ConstraintInstruction::Lc0Add(index) => {
                    lc0 = lc0 + (coeff, variables[index]);
                }
                ConstraintInstruction::Lc1Add(index) => {
                    lc1 = lc1 + (coeff, variables[index]);
                }
                ConstraintInstruction::Lc2Add(index) => {
                    lc2 = lc2 + (coeff, variables[index]);
                }
                ConstraintInstruction::Lc0Sub(index) => {
                    lc0 = lc0 - (coeff, variables[index]);
                }
                ConstraintInstruction::Lc1Sub(index) => {
                    lc1 = lc1 - (coeff, variables[index]);
                }
                ConstraintInstruction::Lc2Sub(index) => {
                    lc2 = lc2 - (coeff, variables[index]);
                }
                ConstraintInstruction::Lc0AddOne => {
                    lc0 = lc0 + CS::one();
                }
                ConstraintInstruction::Lc1AddOne => {
                    lc1 = lc1 + CS::one();
                }
                ConstraintInstruction::Lc2AddOne => {
                    lc2 = lc2 + CS::one();
                }
                ConstraintInstruction::Lc0SubOne => {
                    lc0 = lc0 - CS::one();
                }
                ConstraintInstruction::Lc1SubOne => {
                    lc1 = lc1 - CS::one();
                }
                ConstraintInstruction::Lc2SubOne => {
                    lc2 = lc2 - CS::one();
                }
                ConstraintInstruction::Lc0AddCoeff(const_index, index) => {
                    lc0 = lc0 + (self.constants[const_index], variables[index]);
                }
                ConstraintInstruction::Lc1AddCoeff(const_index, index) => {
                    lc1 = lc1 + (self.constants[const_index], variables[index]);
                }
                ConstraintInstruction::Lc2AddCoeff(const_index, index) => {
                    lc2 = lc2 + (self.constants[const_index], variables[index]);
                }
                ConstraintInstruction::Lc0AddConstant(const_index) => {
                    lc0 = lc0 + (self.constants[const_index], CS::one());
                }
                ConstraintInstruction::Lc1AddConstant(const_index) => {
                    lc1 = lc1 + (self.constants[const_index], CS::one());
                }
                ConstraintInstruction::Lc2AddConstant(const_index) => {
                    lc2 = lc2 + (self.constants[const_index], CS::one());
                }
                ConstraintInstruction::Enforce => {
                    cs.enforce(
                        || "constraint",
                        |_| lc0.clone(),
                        |_| lc1.clone(),
                        |_| lc2.clone(),
                    );
                    coeff = bls12_381::Scalar::one();
                    lc0 = bellman::LinearCombination::<Scalar>::zero();
                    lc1 = bellman::LinearCombination::<Scalar>::zero();
                    lc2 = bellman::LinearCombination::<Scalar>::zero();
                }
                ConstraintInstruction::LcCoeffReset => {
                    coeff = bls12_381::Scalar::one();
                }
                ConstraintInstruction::LcCoeffDouble => {
                    coeff = coeff.double();
                }
            }
        }

        Ok(())
    }
}
