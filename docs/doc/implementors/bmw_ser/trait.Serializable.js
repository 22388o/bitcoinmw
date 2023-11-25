(function() {var implementors = {
"bmw_evh":[["impl Serializable for <a class=\"struct\" href=\"bmw_evh/struct.WriteState.html\" title=\"struct bmw_evh::WriteState\">WriteState</a>"]],
"bmw_ser":[],
"bmw_util":[["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.Pattern.html\" title=\"struct bmw_util::Pattern\">Pattern</a>"],["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.SlabAllocatorConfig.html\" title=\"struct bmw_util::SlabAllocatorConfig\">SlabAllocatorConfig</a>"],["impl&lt;K&gt; <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/1.74.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"bmw_util/trait.Hashset.html\" title=\"trait bmw_util::Hashset\">Hashset</a>&lt;K&gt;&gt;<span class=\"where fmt-newline\">where\n    K: <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> + 'static,</span>"],["impl&lt;K, V&gt; <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/1.74.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"bmw_util/trait.Hashtable.html\" title=\"trait bmw_util::Hashtable\">Hashtable</a>&lt;K, V&gt;&gt;<span class=\"where fmt-newline\">where\n    K: <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> + 'static,\n    V: <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,</span>"],["impl&lt;S: <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&gt; <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.ArrayList.html\" title=\"struct bmw_util::ArrayList\">ArrayList</a>&lt;S&gt;"],["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"enum\" href=\"bmw_util/enum.ConfigOption.html\" title=\"enum bmw_util::ConfigOption\">ConfigOption</a>&lt;'_&gt;"],["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.HashtableConfig.html\" title=\"struct bmw_util::HashtableConfig\">HashtableConfig</a>"],["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.ListConfig.html\" title=\"struct bmw_util::ListConfig\">ListConfig</a>"],["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.HashsetConfig.html\" title=\"struct bmw_util::HashsetConfig\">HashsetConfig</a>"],["impl&lt;S: <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + 'static&gt; <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/1.74.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"bmw_util/trait.SortableList.html\" title=\"trait bmw_util::SortableList\">SortableList</a>&lt;S&gt;&gt;"],["impl&lt;S: <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.Array.html\" title=\"struct bmw_util::Array\">Array</a>&lt;S&gt;"],["impl <a class=\"trait\" href=\"bmw_util/trait.Serializable.html\" title=\"trait bmw_util::Serializable\">Serializable</a> for <a class=\"struct\" href=\"bmw_util/struct.ThreadPoolConfig.html\" title=\"struct bmw_util::ThreadPoolConfig\">ThreadPoolConfig</a>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()