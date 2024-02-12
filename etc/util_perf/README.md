# Util Perf

Util Perf is a tool to compare the performance of serveral data structures. You can build util_perf with the following command:

```text
$ cargo build --release
```

To run util_perf:

```text
$ ./target/release/util_perf --help
```

This will list the options.

# Results

Util Perf results may look like this:

```
$ ./target/release/util_perf --array --vec --hashmap --hashtable
[2024-02-12 12:19:08.060]: util_perf
[2024-02-12 12:19:08.060]: Testing hashtable
[2024-02-12 12:19:08.060]: hashtable init: alloc: 100,096, dealloc: 96, alloc_qty: 2, dealloc_qty: 1, delta: 100,000, elapsed: 56.806µs
[2024-02-12 12:19:08.063]: hashtable insert: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 2.192054ms
[2024-02-12 12:19:08.064]: hashtable get: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 1.350064ms
[2024-02-12 12:19:08.065]: hashtable drop: alloc: 100,000, dealloc: 200,000, alloc_qty: 1, dealloc_qty: 2, delta: -100,000, elapsed: 629.204µs
[2024-02-12 12:19:08.065]: Testing hashmap
[2024-02-12 12:19:08.065]: hashmap init: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 34ns
[2024-02-12 12:19:08.065]: hashmap insert: alloc: 688,252, dealloc: 344,172, alloc_qty: 26, dealloc_qty: 24, delta: 344,080, elapsed: 670.225µs
[2024-02-12 12:19:08.066]: hashmap get: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 188.963µs
[2024-02-12 12:19:08.066]: hashmap drop: alloc: 0, dealloc: 344,080, alloc_qty: 0, dealloc_qty: 2, delta: -344,080, elapsed: 30.794µs
[2024-02-12 12:19:08.066]: testing vec
[2024-02-12 12:19:08.066]: vec init: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 29ns
[2024-02-12 12:19:08.066]: vec insert: alloc: 131,056, dealloc: 65,520, alloc_qty: 13, dealloc_qty: 12, delta: 65,536, elapsed: 16.38µs
[2024-02-12 12:19:08.066]: vec iter: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 29ns
[2024-02-12 12:19:08.066]: vec drop: alloc: 0, dealloc: 65,536, alloc_qty: 0, dealloc_qty: 1, delta: -65,536, elapsed: 122ns
[2024-02-12 12:19:08.066]: Testing array
[2024-02-12 12:19:08.066]: array init: alloc: 40,000, dealloc: 0, alloc_qty: 1, dealloc_qty: 0, delta: 40,000, elapsed: 1.109µs
[2024-02-12 12:19:08.066]: array insert: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 856ns
[2024-02-12 12:19:08.066]: array iter: alloc: 0, dealloc: 0, alloc_qty: 0, dealloc_qty: 0, delta: 0, elapsed: 28ns
[2024-02-12 12:19:08.066]: array drop: alloc: 0, dealloc: 40,000, alloc_qty: 0, dealloc_qty: 1, delta: -40,000, elapsed: 96ns
```

Each data structure tested has metrics for four operations (init = initialization of the data structure, insert = inserting 10,000 elements, get/iter = get elements in the hashmap/hashtable or iterate through the array/vec, drop = calling the drop handler on the data structure). "alloc" is the number of times memory has been allocated, "dealloc" is the number of times memory is deallocated. "alloc/dealloc qty" is the quantity of memory allocated and deallocated. "delta" is the difference between the memory allocated and deallocated. "elapsed" is the time elapsed for the operations. The results indicate that while the performance of our native datastructures is slower, the memory usage is stable and after the data structure is initialized, no allocations/deallocations occur until the data strcture is dropped. For the long-lived data structures we use in the eventhandler and the http server, this means that the memory usage can be perfectly stable for even very high loads. Also, array inserts are significantly faster and iteration is also slightly faster. While inserts/gets are slower for the hashmap, they are within an order of magnitude. Since memory stability is quite important and hash inserts/gets are not typically the bottleneck in an application the use of this hashtable data structure is justified.
