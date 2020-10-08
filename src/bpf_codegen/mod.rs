use crate::evm_opcode::Instruction;
use solana_rbpf::insn_builder::BpfCode;

pub struct BpfCodeGen {
    program: BpfCode,
}

impl BpfCodeGen {
    pub fn new() -> Self {
        let program = BpfCode::new();
        Self {
            program,
        }
    }

    pub fn gen(&mut self, instrs: &[Instruction]) {
        for instr in instrs {
            match instr {
                Instruction::Stop => {}
                Instruction::Add => {}
                Instruction::Mul => {}
                Instruction::Sub => {}
                Instruction::Div => {}
                Instruction::SDiv => {}
                Instruction::Mod => {}
                Instruction::SMod => {}
                Instruction::AddMod => {}
                Instruction::MulMod => {}
                Instruction::Exp => {}
                Instruction::SignExtend => {}
                Instruction::Lt => {}
                Instruction::Gt => {}
                Instruction::SLt => {}
                Instruction::SGt => {}
                Instruction::EQ => {}
                Instruction::IsZero => {}
                Instruction::And => {}
                Instruction::Or => {}
                Instruction::Xor => {}
                Instruction::Not => {}
                Instruction::Byte => {}
                Instruction::Shl => {}
                Instruction::Shr => {}
                Instruction::Sar => {}
                Instruction::Sha3 => {}
                Instruction::Addr => {}
                Instruction::Balance => {}
                Instruction::Origin => {}
                Instruction::Caller => {}
                Instruction::CallValue => {}
                Instruction::CallDataLoad => {}
                Instruction::CallDataSize => {}
                Instruction::CallDataCopy => {}
                Instruction::CodeSize => {}
                Instruction::CodeCopy => {}
                Instruction::GasPrice => {}
                Instruction::ExtCodeSize => {}
                Instruction::ExtCodeCopy => {}
                Instruction::ReturnDataSize => {}
                Instruction::ReturnDataCopy => {}
                Instruction::ExtCodeHash => {}
                Instruction::Blockhash => {}
                Instruction::Coinbase => {}
                Instruction::Timestamp => {}
                Instruction::Number => {}
                Instruction::Difficulty => {}
                Instruction::GasLimit => {}
                Instruction::Pop => {}
                Instruction::MLoad => {}
                Instruction::MStore => {}
                Instruction::MStore8 => {}
                Instruction::SLoad => {}
                Instruction::SStore => {}
                Instruction::Jump => {}
                Instruction::JumpIf => {}
                Instruction::PC => {}
                Instruction::MSize => {}
                Instruction::Gas => {}
                Instruction::JumpDest => {}
                Instruction::Push(_) => {}
                Instruction::Dup(_) => {}
                Instruction::Swap(_) => {}
                Instruction::Log(_) => {}
                Instruction::Create => {}
                Instruction::Call => {}
                Instruction::CallCode => {}
                Instruction::Return => {}
                Instruction::DelegateCall => {}
                Instruction::Create2 => {}
                Instruction::Revert => {}
                Instruction::StaticCall => {}
                Instruction::Invalid => {}
                Instruction::SelfDestruct => {}
            }
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codegen() {
        let mut codegen = BpfCodeGen::new();
        codegen.gen(&[Instruction::Push(vec![0])]);
    }
}
