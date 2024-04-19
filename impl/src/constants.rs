// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub(crate) const DOC_REPLACEMENTS: [(&str, &str); 15] = [
	(" bool ", " `[`prim@bool`]` "),
	(" () ", " `[`unit`]` "),
	(" Result ", " `[`Result`]` "),
	(" Error ", " `[`Error`]` "),
	(" usize ", " `[`prim@usize`]` "),
	(" u8 ", " `[`prim@u8`]` "),
	(" u16 ", " `[`prim@u16`]` "),
	(" u32 ", " `[`prim@u32`]` "),
	(" u64 ", " `[`prim@u64`]` "),
	(" u128 ", " `[`prim@u128`]` "),
	(" String ", " `[`String`]` "),
	(" Vec ", " `[`Vec`]` "),
	(" Box ", " `[`Box`]` "),
	(" Send ", " `[`Send`]` "),
	(" Sync ", " `[`Sync`]` "),
];
