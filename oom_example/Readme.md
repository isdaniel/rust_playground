setting

```
export MALLOC_CONF=prof:true,prof_active:true,prof_final:true,lg_prof_sample:0
```

pdf

```
jeprof --show_bytes --pdf ./target/debug/oom_example ./profile.out > test.pdf
```

text

```
 jeprof --show_bytes --text ./target/debug/oom_example ./profile.out 

 Using local file ./target/debug/oom_example.
Using local file ./profile.out.
Total: 2054144 B
 2048000  99.7%  99.7%  2048000  99.7% ::alloc
    6144   0.3% 100.0%     6144   0.3% ::realloc
       0   0.0% 100.0%  2048000  99.7% ::allocate
       0   0.0% 100.0%  2054144 100.0% __libc_start_call_main
       0   0.0% 100.0%  2054144 100.0% __libc_start_main_impl
       0   0.0% 100.0%  2048000  99.7% __rustc::__rust_alloc
       0   0.0% 100.0%     6144   0.3% __rustc::__rust_realloc
       0   0.0% 100.0%  2054144 100.0% _start
       0   0.0% 100.0%  2048000  99.7% alloc::alloc::Global::alloc_impl
       0   0.0% 100.0%  2048000  99.7% alloc::alloc::alloc
       0   0.0% 100.0%     6144   0.3% alloc::raw_vec::RawVec::grow_one
       0   0.0% 100.0%     6144   0.3% alloc::raw_vec::RawVecInner::grow_amortized
       0   0.0% 100.0%     6144   0.3% alloc::raw_vec::RawVecInner::grow_one (inline)
       0   0.0% 100.0%     6144   0.3% alloc::raw_vec::finish_grow
       0   0.0% 100.0%     6144   0.3% alloc::vec::Vec::push
       0   0.0% 100.0%  2054144 100.0% core::ops::function::FnOnce::call_once
       0   0.0% 100.0%  2054144 100.0% core::ops::function::impls::::call_once (inline)
       0   0.0% 100.0%  2048000  99.7% hashbrown::map::HashMap::with_capacity_and_hasher
       0   0.0% 100.0%  2048000  99.7% hashbrown::raw::RawTable::with_capacity (inline)
       0   0.0% 100.0%  2048000  99.7% hashbrown::raw::RawTable::with_capacity_in
       0   0.0% 100.0%  2048000  99.7% hashbrown::raw::RawTableInner::fallible_with_capacity
       0   0.0% 100.0%  2048000  99.7% hashbrown::raw::RawTableInner::new_uninitialized
       0   0.0% 100.0%  2048000  99.7% hashbrown::raw::RawTableInner::with_capacity (inline)
       0   0.0% 100.0%  2048000  99.7% hashbrown::raw::alloc::inner::do_alloc (inline)
       0   0.0% 100.0%  2054144 100.0% main
       0   0.0% 100.0%  2054144 100.0% oom_example::main
       0   0.0% 100.0%  2048000  99.7% std::collections::hash::map::HashMap::with_capacity
       0   0.0% 100.0%  2048000  99.7% std::collections::hash::map::HashMap::with_capacity_and_hasher (inline)
       0   0.0% 100.0%  2054144 100.0% std::panic::catch_unwind (inline)
       0   0.0% 100.0%  2054144 100.0% std::panicking::try (inline)
       0   0.0% 100.0%  2054144 100.0% std::panicking::try::do_call (inline)
       0   0.0% 100.0%  2054144 100.0% std::rt::lang_start
       0   0.0% 100.0%  2054144 100.0% std::rt::lang_start::{{closure}}
       0   0.0% 100.0%  2054144 100.0% std::rt::lang_start_internal
       0   0.0% 100.0%  2054144 100.0% std::rt::lang_start_internal::{{closure}} (inline)
       0   0.0% 100.0%  2054144 100.0% std::sys::backtrace::__rust_begin_short_backtrace
```

