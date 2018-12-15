// Copyright 2018 Grove Enterprises LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::rc::Rc;
use std::sync::Arc;

use arrow::array::*;
use arrow::array_ops;
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;

use super::super::logicalplan::{Expr, Operator};
use super::context::ExecutionContext;
use super::error::{ExecutionError, Result};

/// Compiled Expression (basically just a closure to evaluate the expression at runtime)
pub type CompiledExpr = Rc<Fn(&RecordBatch) -> Result<ArrayRef>>;

pub type CompiledCastFunction = Rc<Fn(&ArrayRef) -> Result<ArrayRef>>;

pub enum AggregateType {
    Min,
    Max,
    Sum,
    Count,
    Avg,
    //CountDistinct()
}

/// Runtime expression
pub enum RuntimeExpr {
    Compiled {
        f: CompiledExpr,
        t: DataType,
    },
    AggregateFunction {
        f: AggregateType,
        args: Vec<CompiledExpr>,
        t: DataType,
    },
}

impl RuntimeExpr {
    pub fn get_func(&self) -> CompiledExpr {
        match self {
            &RuntimeExpr::Compiled { ref f, .. } => f.clone(),
            _ => panic!(),
        }
    }
    pub fn get_type(&self) -> DataType {
        match self {
            &RuntimeExpr::Compiled { ref t, .. } => t.clone(),
            &RuntimeExpr::AggregateFunction { ref t, .. } => t.clone(),
        }
    }
}

/// Compiles a scalar expression into a closure
pub fn compile_expr(
    ctx: Rc<ExecutionContext>,
    expr: &Expr,
    input_schema: &Schema,
) -> Result<RuntimeExpr> {
    match *expr {
        Expr::AggregateFunction {
            ref name,
            ref args,
            ref return_type,
        } => {
            assert_eq!(1, args.len());

            let compiled_args: Result<Vec<RuntimeExpr>> = args
                .iter()
                .map(|e| compile_scalar_expr(&ctx, e, input_schema))
                .collect();

            let func = match name.to_lowercase().as_ref() {
                "min" => AggregateType::Min,
                "max" => AggregateType::Max,
                "count" => AggregateType::Count,
                "sum" => AggregateType::Sum,
                _ => unimplemented!("Unsupported aggregate function '{}'", name),
            };

            Ok(RuntimeExpr::AggregateFunction {
                f: func,
                args: compiled_args?
                    .iter()
                    .map(|e| e.get_func().clone())
                    .collect(),
                t: return_type.clone(),
            })
        }
        _ => Ok(compile_scalar_expr(&ctx, expr, input_schema)?),
    }
}

macro_rules! binary_op {
    ($LEFT:expr, $RIGHT:expr, $OP:ident, $DT:ident) => {{
        let ll = $LEFT.as_any().downcast_ref::<$DT>().unwrap();
        let rr = $RIGHT.as_any().downcast_ref::<$DT>().unwrap();
        Ok(Arc::new(array_ops::$OP(&ll, &rr)?))
    }};
}

macro_rules! math_ops {
    ($LEFT:expr, $RIGHT:expr, $BATCH:expr, $OP:ident) => {{
        let left_values = $LEFT.get_func()($BATCH)?;
        let right_values = $RIGHT.get_func()($BATCH)?;
        match (left_values.data_type(), right_values.data_type()) {
            (DataType::Int8, DataType::Int8) => {
                binary_op!(left_values, right_values, $OP, Int8Array)
            }
            (DataType::Int16, DataType::Int16) => {
                binary_op!(left_values, right_values, $OP, Int16Array)
            }
            (DataType::Int32, DataType::Int32) => {
                binary_op!(left_values, right_values, $OP, Int32Array)
            }
            (DataType::Int64, DataType::Int64) => {
                binary_op!(left_values, right_values, $OP, Int64Array)
            }
            (DataType::UInt8, DataType::UInt8) => {
                binary_op!(left_values, right_values, $OP, UInt8Array)
            }
            (DataType::UInt16, DataType::UInt16) => {
                binary_op!(left_values, right_values, $OP, UInt16Array)
            }
            (DataType::UInt32, DataType::UInt32) => {
                binary_op!(left_values, right_values, $OP, UInt32Array)
            }
            (DataType::UInt64, DataType::UInt64) => {
                binary_op!(left_values, right_values, $OP, UInt64Array)
            }
            (DataType::Float32, DataType::Float32) => {
                binary_op!(left_values, right_values, $OP, Float32Array)
            }
            (DataType::Float64, DataType::Float64) => {
                binary_op!(left_values, right_values, $OP, Float64Array)
            }
            _ => Err(ExecutionError::NotImplemented),
        }
    }};
}

macro_rules! comparison_ops {
    ($LEFT:expr, $RIGHT:expr, $BATCH:expr, $OP:ident) => {{
        let left_values = $LEFT.get_func()($BATCH)?;
        let right_values = $RIGHT.get_func()($BATCH)?;
        match (left_values.data_type(), right_values.data_type()) {
            (DataType::Int8, DataType::Int8) => {
                binary_op!(left_values, right_values, $OP, Int8Array)
            }
            (DataType::Int16, DataType::Int16) => {
                binary_op!(left_values, right_values, $OP, Int16Array)
            }
            (DataType::Int32, DataType::Int32) => {
                binary_op!(left_values, right_values, $OP, Int32Array)
            }
            (DataType::Int64, DataType::Int64) => {
                binary_op!(left_values, right_values, $OP, Int64Array)
            }
            (DataType::UInt8, DataType::UInt8) => {
                binary_op!(left_values, right_values, $OP, UInt8Array)
            }
            (DataType::UInt16, DataType::UInt16) => {
                binary_op!(left_values, right_values, $OP, UInt16Array)
            }
            (DataType::UInt32, DataType::UInt32) => {
                binary_op!(left_values, right_values, $OP, UInt32Array)
            }
            (DataType::UInt64, DataType::UInt64) => {
                binary_op!(left_values, right_values, $OP, UInt64Array)
            }
            (DataType::Float32, DataType::Float32) => {
                binary_op!(left_values, right_values, $OP, Float32Array)
            }
            (DataType::Float64, DataType::Float64) => {
                binary_op!(left_values, right_values, $OP, Float64Array)
            }
            //TODO other types
            _ => Err(ExecutionError::NotImplemented),
        }
    }};
}

/// Compiles a scalar expression into a closure
pub fn compile_scalar_expr(
    ctx: &ExecutionContext,
    expr: &Expr,
    input_schema: &Schema,
) -> Result<RuntimeExpr> {
    match expr {
        &Expr::Literal(ref _lit) => {
            Err(ExecutionError::NotImplemented)
            //            let literal_value = lit.clone();
            //            Ok(RuntimeExpr::Compiled {
            //                f: Rc::new(move |_| {
            //                    // literal values are a bit special - we don't repeat them in a vector
            //                    // because it would be redundant, so we have a single value in a vector instead
            //                    Ok(Value::Scalar(Rc::new(literal_value.clone())))
            //                }),
            //                t: DataType::Float64, //TODO
            //            })
        }
        &Expr::Column(index) => Ok(RuntimeExpr::Compiled {
            f: Rc::new(move |batch: &RecordBatch| Ok((*batch.column(index)).clone())),
            t: input_schema.field(index).data_type().clone(),
        }),
        &Expr::Cast { ref expr, .. } => match expr.as_ref() {
            &Expr::Column(_index) => {
                Err(ExecutionError::NotImplemented)
                //                let compiled_cast_expr = compile_cast_column(data_type.clone())?;
                //                Ok(RuntimeExpr::Compiled {
                //                    f: Rc::new(move |batch: &RecordBatch| {
                //                        (compiled_cast_expr)(batch.column(index))
                //                    }),
                //                    t: data_type.clone(),
                //                })
            }
            &Expr::Literal(ref _lit) => {
                Err(ExecutionError::NotImplemented)
                //                let compiled_cast_expr = compile_cast_scalar(lit, data_type)?;
                //                Ok(RuntimeExpr::Compiled {
                //                    f: Rc::new(move |_: &RecordBatch| {
                //                        (compiled_cast_expr)(&Value::Scalar(Rc::new(ScalarValue::Null))) // pointless arg
                //                    }),
                //                    t: data_type.clone(),
                //                })
            }
            other => Err(ExecutionError::General(format!(
                "CAST not implemented for expression {:?}",
                other
            ))),
        },
        //        &Expr::IsNotNull(ref expr) => {
        //            let compiled_expr = compile_scalar_expr(ctx, expr, input_schema)?;
        //            Ok(RuntimeExpr::Compiled {
        //                f: Rc::new(move |batch: &RecordBatch| {
        //                    let left_values = compiled_expr.get_func()(batch)?;
        //                    left_values.is_not_null()
        //                }),
        //                t: DataType::Boolean,
        //            })
        //        }
        //        &Expr::IsNull(ref expr) => {
        //            let compiled_expr = compile_scalar_expr(ctx, expr, input_schema)?;
        //            Ok(RuntimeExpr::Compiled {
        //                f: Rc::new(move |batch: &RecordBatch| {
        //                    let left_values = compiled_expr.get_func()(batch)?;
        //                    left_values.is_null()
        //                }),
        //                t: DataType::Boolean,
        //            })
        //        }
        &Expr::BinaryExpr {
            ref left,
            ref op,
            ref right,
        } => {
            let left_expr = compile_scalar_expr(ctx, left, input_schema)?;
            let right_expr = compile_scalar_expr(ctx, right, input_schema)?;
            let op_type = left_expr.get_type().clone();
            match op {
                &Operator::Eq => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        comparison_ops!(left_expr, right_expr, batch, eq)
                    }),
                    t: DataType::Boolean,
                }),
                &Operator::NotEq => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        comparison_ops!(left_expr, right_expr, batch, neq)
                    }),
                    t: DataType::Boolean,
                }),
                &Operator::Lt => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        comparison_ops!(left_expr, right_expr, batch, lt)
                    }),
                    t: DataType::Boolean,
                }),
                &Operator::LtEq => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        comparison_ops!(left_expr, right_expr, batch, lt_eq)
                    }),
                    t: DataType::Boolean,
                }),
                &Operator::Gt => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        comparison_ops!(left_expr, right_expr, batch, gt)
                    }),
                    t: DataType::Boolean,
                }),
                &Operator::GtEq => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        comparison_ops!(left_expr, right_expr, batch, gt_eq)
                    }),
                    t: DataType::Boolean,
                }),
                //                    &Operator::And => Ok(RuntimeExpr::Compiled {
                //                        f: Rc::new(move |batch: &RecordBatch| {
                //                            let left_values = left_expr.get_func()(batch)?;
                //                            let right_values = right_expr.get_func()(batch)?;
                //                            left_values.and(&right_values)
                //                        }),
                //                        t: DataType::Boolean,
                //                    }),
                //                    &Operator::Or => Ok(RuntimeExpr::Compiled {
                //                        f: Rc::new(move |batch: &RecordBatch| {
                //                            let left_values = left_expr.get_func()(batch)?;
                //                            let right_values = right_expr.get_func()(batch)?;
                //                            left_values.or(&right_values)
                //                        }),
                //                        t: DataType::Boolean,
                //                    }),
                &Operator::Plus => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        math_ops!(left_expr, right_expr, batch, add)
                    }),
                    t: op_type,
                }),
                &Operator::Minus => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        math_ops!(left_expr, right_expr, batch, subtract)
                    }),
                    t: op_type,
                }),
                &Operator::Multiply => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        math_ops!(left_expr, right_expr, batch, multiply)
                    }),
                    t: op_type,
                }),
                &Operator::Divide => Ok(RuntimeExpr::Compiled {
                    f: Rc::new(move |batch: &RecordBatch| {
                        math_ops!(left_expr, right_expr, batch, divide)
                    }),
                    t: op_type,
                }),
                _ => Err(ExecutionError::NotImplemented),
            }
        }
        _ => Err(ExecutionError::NotImplemented),
    }
}
