// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

use crate::{CircuitType, CircuitVariableType, FunctionType, ImportedSymbols, ParameterType, SymbolTableError};
use leo_core::CorePackageList;
use leo_imports::ImportParser;
use leo_typed::{Circuit, Function, Identifier, ImportStatement, ImportSymbol, Input, Package, Program};

use std::collections::HashMap;

pub const INPUT_VARIABLE_NAME: &str = "input";
pub const RECORD_VARIABLE_NAME: &str = "record";
pub const REGISTERS_VARIABLE_NAME: &str = "registers";
pub const STATE_VARIABLE_NAME: &str = "state";
pub const STATE_LEAF_VARIABLE_NAME: &str = "state_leaf";

/// A abstract data type that builds symbol tables for functions and circuits
///
/// A symbol table has access to all function and circuit names in its
/// parent's symbol table.
/// A symbol table cannot access names in its child's symbol table.
/// Children cannot access names in another sibling's symbol table.
#[derive(Clone)]
pub struct SymbolTable {
    /// Maps name -> parameter type.
    names: HashMap<String, ParameterType>,

    /// Maps circuit name -> circuit type.
    circuits: HashMap<String, CircuitType>,

    /// Maps function name -> function type.
    functions: HashMap<String, FunctionType>,

    /// The parent of this symbol table.
    parent: Option<Box<SymbolTable>>,
}

impl SymbolTable {
    ///
    /// Creates a new symbol table with a given parent symbol table.
    ///
    pub fn new(parent: Option<Box<SymbolTable>>) -> Self {
        SymbolTable {
            names: HashMap::new(),
            circuits: HashMap::new(),
            functions: HashMap::new(),
            parent,
        }
    }

    ///
    /// Insert a function or circuit name into the symbol table from a given name and variable type.
    ///
    /// If the symbol table did not have this name present, `None` is returned.
    /// If the symbol table did have this name present, the variable type is updated, and the old
    /// variable type is returned.
    ///
    pub fn insert_name(&mut self, name: String, variable_type: ParameterType) -> Option<ParameterType> {
        self.names.insert(name, variable_type)
    }

    ///
    /// Insert a circuit name into the symbol table from a given name and variable type.
    ///
    /// Returns an error if the circuit name is a duplicate.
    ///
    pub fn insert_circuit_name(&mut self, name: String, variable_type: ParameterType) -> Result<(), SymbolTableError> {
        // Check that the circuit name is unique.
        match self.insert_name(name, variable_type) {
            Some(duplicate) => Err(SymbolTableError::duplicate_circuit(duplicate)),
            None => Ok(()),
        }
    }

    ///
    /// Insert a function name into the symbol table from a given name and variable type.
    ///
    /// Returns an error if the function name is a duplicate.
    ///
    pub fn insert_function_name(&mut self, name: String, variable_type: ParameterType) -> Result<(), SymbolTableError> {
        // Check that the circuit name is unique.
        match self.insert_name(name, variable_type) {
            Some(duplicate) => Err(SymbolTableError::duplicate_function(duplicate)),
            None => Ok(()),
        }
    }

    ///
    /// Insert a circuit definition into the symbol table from a given circuit identifier and
    /// circuit type.
    ///
    /// If the symbol table did not have this name present, `None` is returned.
    /// If the symbol table did have this name present, the circuit type is updated, and the old
    /// circuit type is returned.
    ///
    pub fn insert_circuit(&mut self, identifier: Identifier, circuit_type: CircuitType) -> Option<CircuitType> {
        self.circuits.insert(identifier.name, circuit_type)
    }

    ///
    /// Insert a function definition into the symbol table from a given identifier and
    /// function type.
    ///
    /// If the symbol table did not have this name present, `None` is returned.
    /// If the symbol table did have this name present, the function type is updated, and the old
    /// function type is returned.
    ///
    pub fn insert_function(&mut self, identifier: Identifier, function_type: FunctionType) -> Option<FunctionType> {
        self.functions.insert(identifier.name, function_type)
    }

    ///
    /// Returns a reference to the variable type corresponding to the name.
    ///
    /// If the symbol table did not have this name present, then `None` is returned.
    ///
    pub fn get_variable(&self, name: &String) -> Option<&ParameterType> {
        // Lookup variable name in symbol table.
        match self.names.get(name) {
            Some(variable) => Some(variable),
            None => None,
        }
    }

    ///
    /// Returns a reference to the circuit type corresponding to the name.
    ///
    /// If the symbol table did not have this name present, then the parent symbol table is checked.
    /// If there is no parent symbol table, then `None` is returned.
    ///
    pub fn get_circuit(&self, name: &String) -> Option<&CircuitType> {
        // Lookup name in symbol table.
        match self.circuits.get(name) {
            Some(circuit) => Some(circuit),
            None => {
                // Lookup name in parent symbol table.
                match &self.parent {
                    Some(parent) => parent.get_circuit(name),
                    None => None,
                }
            }
        }
    }

    ///
    /// Returns a reference to the function type corresponding to the name.
    ///
    /// If the symbol table did not have this name present, then the parent symbol table is checked.
    /// If there is no parent symbol table, then `None` is returned.
    ///
    pub fn get_function(&self, key: &String) -> Option<&FunctionType> {
        // Lookup name in symbol table.
        match self.functions.get(key) {
            Some(circuit) => Some(circuit),
            None => {
                // Lookup name in parent symbol table
                match &self.parent {
                    Some(parent) => parent.get_function(key),
                    None => None,
                }
            }
        }
    }

    ///
    /// Inserts function input types into symbol table.
    ///
    /// Creates a new `CircuitType` to represent the input values.
    /// The new type contains register, record, state, and state leaf circuit variables.
    /// This allows easy access to input types using dot syntax: `input.register.r0`.
    ///
    pub fn insert_input(&mut self, input: &Input) -> Result<(), SymbolTableError> {
        // Get values for each input section.
        let registers_values = input.get_registers().values();
        let record_values = input.get_record().values();
        let state_values = input.get_state().values();
        let state_leaf_values = input.get_state_leaf().values();

        // Create a new `CircuitType` for each input section.
        let registers_type =
            CircuitType::from_input_section(&self, REGISTERS_VARIABLE_NAME.to_string(), registers_values)?;
        let record_type = CircuitType::from_input_section(&self, RECORD_VARIABLE_NAME.to_string(), record_values)?;
        let state_type = CircuitType::from_input_section(&self, STATE_VARIABLE_NAME.to_string(), state_values)?;
        let state_leaf_type =
            CircuitType::from_input_section(&self, STATE_LEAF_VARIABLE_NAME.to_string(), state_leaf_values)?;

        // Create a new `CircuitVariableType` for each type.
        let registers_variable = CircuitVariableType::from(&registers_type);
        let record_variable = CircuitVariableType::from(&record_type);
        let state_variable = CircuitVariableType::from(&state_type);
        let state_leaf_variable = CircuitVariableType::from(&state_leaf_type);

        // Create new `CircuitType` for input keyword.
        let input_type = CircuitType {
            identifier: Identifier::new(INPUT_VARIABLE_NAME.to_string()),
            variables: vec![registers_variable, record_variable, state_variable, state_leaf_variable],
            functions: Vec::new(),
        };

        // Insert each circuit type into the symbol table.
        self.insert_circuit(registers_type.identifier.clone(), registers_type);
        self.insert_circuit(record_type.identifier.clone(), record_type);
        self.insert_circuit(state_type.identifier.clone(), state_type);
        self.insert_circuit(state_leaf_type.identifier.clone(), state_leaf_type);
        self.insert_circuit(input_type.identifier.clone(), input_type);

        Ok(())
    }

    ///
    /// Inserts the imported symbol into the symbol table if it is present in the given program.
    ///
    pub fn insert_import_symbol(&mut self, symbol: ImportSymbol, program: &Program) -> Result<(), SymbolTableError> {
        // Check for import *.
        if symbol.is_star() {
            // Insert all program circuits.
            self.check_duplicate_circuits(&program.circuits)?;

            // Insert all program functions.
            self.check_duplicate_functions(&program.functions)
        } else {
            // Check for a symbol alias.
            let identifier = symbol.alias.to_owned().unwrap_or(symbol.symbol.to_owned());

            // Check if the imported symbol is a circuit
            let matched_circuit = program
                .circuits
                .iter()
                .find(|(circuit_name, _circuit_def)| symbol.symbol == **circuit_name);

            match matched_circuit {
                Some((_circuit_name, circuit)) => {
                    // Insert imported circuit.
                    self.insert_circuit_name(identifier.to_string(), ParameterType::from(circuit.to_owned()))
                }
                None => {
                    // Check if the imported symbol is a function.
                    let matched_function = program
                        .functions
                        .iter()
                        .find(|(function_name, _function)| symbol.symbol == **function_name);

                    match matched_function {
                        Some((_function_name, function)) => {
                            // Insert the imported function.
                            self.insert_function_name(identifier.to_string(), ParameterType::from(function.to_owned()))
                        }
                        None => Err(SymbolTableError::unknown_symbol(&symbol, program)),
                    }
                }
            }
        }
    }

    ///
    /// Inserts one or more imported symbols for a given import statement.
    ///
    /// No type resolution performed at this step.
    ///
    pub fn insert_import(
        &mut self,
        import: &ImportStatement,
        import_parser: &ImportParser,
    ) -> Result<(), SymbolTableError> {
        // Get imported symbols from statement.
        let imported_symbols = ImportedSymbols::from(import);

        // Import all symbols from an imported file for now.

        // Keep track of which import files have already been checked.
        let mut checked = Vec::new();

        // Iterate over each imported symbol.
        for (name, symbol) in imported_symbols.symbols {
            // Skip the imported symbol if we have already checked the file.
            if checked.contains(&name) {
                continue;
            };

            // Find the imported program.
            let program = import_parser
                .get_import(&name)
                .ok_or_else(|| SymbolTableError::unknown_package(&name, &symbol.span))?;

            // Check the imported program for duplicate types.
            self.check_duplicate_program(program, import_parser)?;

            // Check the imported program for undefined types.
            self.check_unknown_types_program(program)?;

            // Push the imported file's name to checked import files.
            checked.push(name);

            // Store the imported symbol.
            // self.insert_import_symbol(symbol, program)?; // TODO (collinc97) uncomment this line when public/private import scopes are implemented.
        }

        Ok(())
    }

    ///
    /// Inserts core package name and type information into the symbol table.
    ///
    pub fn insert_core_package(&mut self, package: &Package) -> Result<(), SymbolTableError> {
        // Create list of imported core packages.
        let list = CorePackageList::from_package_access(package.access.to_owned())?;

        // Fetch core package symbols from `leo-core`.
        let symbol_list = list.to_symbols()?;

        // Insert name and type information for each core package symbol.
        for (name, circuit) in symbol_list.symbols() {
            // Store name of symbol.
            self.insert_circuit_name(name.to_string(), ParameterType::from(circuit.clone()))?;

            // Create new circuit type for symbol.
            let circuit_type = CircuitType::new(&self, circuit.to_owned())?;

            // Insert circuit type of symbol.
            self.insert_circuit(circuit_type.identifier.clone(), circuit_type);
        }

        Ok(())
    }

    ///
    /// Checks that a given import statement contains imported names that exist in the list of
    /// imported programs.
    ///
    /// Additionally checks for duplicate imported names in the given vector of imports.
    /// Types defined later in the program cannot have the same name.
    ///
    pub fn check_import(
        &mut self,
        import: &ImportStatement,
        import_parser: &ImportParser,
    ) -> Result<(), SymbolTableError> {
        // Check if the import name exists as core package.
        let core_package = import_parser.get_core_package(&import.package);

        // If the core package exists, then attempt to insert the import into the symbol table.
        if let Some(package) = core_package {
            return self.insert_core_package(package);
        }

        // Attempt to insert the imported names into the symbol table.
        self.insert_import(import, import_parser)
    }

    ///
    /// Checks that all given imported names exist in the list of imported programs.
    ///
    /// Additionally checks for duplicate imported names in the given vector of imports.
    /// Types defined later in the program cannot have the same name.
    ///
    pub fn check_imports(
        &mut self,
        imports: &Vec<ImportStatement>,
        import_parser: &ImportParser,
    ) -> Result<(), SymbolTableError> {
        // Iterate over imported names.
        for import in imports.iter() {
            self.check_import(import, import_parser)?;
        }

        Ok(())
    }

    ///
    /// Checks for duplicate import, circuit, and function names given a program.
    ///
    /// If a circuit or function name has no duplicates, then it is inserted into the symbol table.
    /// Variables defined later in the unresolved program cannot have the same name.
    ///
    pub fn check_duplicate_program(
        &mut self,
        program: &Program,
        import_parser: &ImportParser,
    ) -> Result<(), SymbolTableError> {
        // Check unresolved program import names.
        self.check_imports(&program.imports, import_parser)?;

        // Check unresolved program circuit names.
        self.check_duplicate_circuits(&program.circuits)?;

        // Check unresolved program function names.
        self.check_duplicate_functions(&program.functions)?;

        Ok(())
    }

    ///
    /// Checks for duplicate circuit names given a hashmap of circuits.
    ///
    /// If a circuit name has no duplicates, then it is inserted into the symbol table.
    /// Types defined later in the program cannot have the same name.
    ///
    pub fn check_duplicate_circuits(
        &mut self,
        circuits: &HashMap<Identifier, Circuit>,
    ) -> Result<(), SymbolTableError> {
        // Iterate over circuit names and definitions.
        for (identifier, circuit) in circuits.iter() {
            // Attempt to insert the circuit name into the symbol table.
            self.insert_circuit_name(identifier.to_string(), ParameterType::from(circuit.clone()))?;
        }

        Ok(())
    }

    ///
    /// Checks for duplicate function names given a hashmap of functions.
    ///
    /// If a function name has no duplicates, then it is inserted into the symbol table.
    /// Types defined later in the program cannot have the same name.
    ///
    pub fn check_duplicate_functions(
        &mut self,
        functions: &HashMap<Identifier, Function>,
    ) -> Result<(), SymbolTableError> {
        // Iterate over function names and definitions.
        for (identifier, function) in functions.iter() {
            // Attempt to insert the function name into the symbol table.
            self.insert_function_name(identifier.to_string(), ParameterType::from(function.clone()))?;
        }

        Ok(())
    }

    ///
    /// Checks for unknown types in circuit and function definitions given an unresolved program.
    ///
    /// If a circuit or function definition only contains known types, then it is inserted into the
    /// symbol table. Variables defined later in the unresolved program can lookup the definition and
    /// refer to its expected types.
    ///
    pub fn check_unknown_types_program(&mut self, program: &Program) -> Result<(), SymbolTableError> {
        // Check unresolved program circuit definitions.
        self.check_unknown_types_circuits(&program.circuits)?;

        // Check unresolved program function definitions.
        self.check_unknown_types_functions(&program.functions)?;

        Ok(())
    }

    ///
    /// Checks for unknown types in a circuit given a hashmap of circuits.
    ///
    /// If a circuit definition only contains known types, then it is inserted into the
    /// symbol table. Variables defined later in the program can lookup the definition
    /// and refer to its expected types
    ///
    pub fn check_unknown_types_circuits(
        &mut self,
        circuits: &HashMap<Identifier, Circuit>,
    ) -> Result<(), SymbolTableError> {
        // Iterate over circuit names and definitions.
        for (_, circuit) in circuits.iter() {
            // Get the identifier of the circuit.
            let identifier = circuit.circuit_name.clone();

            // Resolve unknown types in the circuit definition.
            let circuit_type = CircuitType::new(self, circuit.clone())?;

            // Attempt to insert the circuit definition into the symbol table.
            self.insert_circuit(identifier, circuit_type);
        }

        Ok(())
    }

    ///
    /// Checks for unknown types in a function given a hashmap of functions.
    ///
    /// If a function definition only contains known types, then it is inserted into the
    /// symbol table. Variables defined later in the program can lookup the definition
    /// and refer to its expected types
    ///
    pub fn check_unknown_types_functions(
        &mut self,
        functions: &HashMap<Identifier, Function>,
    ) -> Result<(), SymbolTableError> {
        // Iterate over function names and definitions.
        for (_, function) in functions.iter() {
            // Get the identifier of the function.
            let identifier = function.identifier.clone();

            // Resolve unknown types in the function definition.
            let function_type = FunctionType::new(&self, function.clone())?;

            // Attempt to insert the function definition into the symbol table.
            self.insert_function(identifier, function_type);
        }

        Ok(())
    }
}
