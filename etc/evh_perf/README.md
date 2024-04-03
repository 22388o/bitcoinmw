# EVH Perf

EVH Perf is a tool that tests the performance of the eventhandler. The tool can be run in either `eventhandler` mode or `client` mode. Additionally, there is a `connect` mode which can be used to test large numbers of connections to an eventhandler. Both `client` and `eventhandler` modes may be run at the same time as well by specifying both the -c and the -e option. The --help option lists all available configuration options. The output of the tool running in eventhandler mode might look like this:

```
$ ./target/release/evh_perf -e --debug --host 0.0.0.0 --max_handles_per_thread 1000 --port 8082 --read_slab_count 500 --reuse_port --threads 10 --tls
[2024-02-08 10:27:00.583]: evh_perf EventHandler/0.0.3-beta.1
----------------------------------------------------------------------------------------------------
[2024-02-08 10:27:00.583]: debug:                  'true'
[2024-02-08 10:27:00.583]: host:                   '0.0.0.0'
[2024-02-08 10:27:00.583]: max_handles_per_thread: '1,000'
[2024-02-08 10:27:00.583]: port:                   '8082'
[2024-02-08 10:27:00.583]: read_slab_count:        '500'
[2024-02-08 10:27:00.583]: reuse_port:             'true'
[2024-02-08 10:27:00.583]: threads:                '10'
[2024-02-08 10:27:00.583]: tls:                    'true'
----------------------------------------------------------------------------------------------------
[2024-02-08 10:27:00.603]: (INFO) Server started in 26 ms.
```

# Building and executing.

To run the evh_perf tool in client mode with the coresponding eventhandler (above), the following options might be specified:

```
$ ./target/release/evh_perf -c --host 127.0.0.1 --max_handles_per_thread 100 --port 8082 --tls --read_slab_count 500 --threads 2 --itt 2 --count 2 --clients 2 --histo --min 20 --max 30 --reconns 2 --sleep 10
[2024-02-08 10:33:19.613]: evh_perf Client/0.0.3-beta.1
----------------------------------------------------------------------------------------------------
[2024-02-08 10:33:19.613]: clients:                '2'
[2024-02-08 10:33:19.613]: count:                  '2'
[2024-02-08 10:33:19.613]: debug:                  'false'
[2024-02-08 10:33:19.613]: histo:                  'true'
[2024-02-08 10:33:19.613]: histo_delta_micros:     '10'
[2024-02-08 10:33:19.613]: host:                   '127.0.0.1'
[2024-02-08 10:33:19.613]: iterations:             '2'
[2024-02-08 10:33:19.613]: max:                    '30'
[2024-02-08 10:33:19.613]: max_handles_per_thread: '100'
[2024-02-08 10:33:19.613]: min:                    '20'
[2024-02-08 10:33:19.613]: port:                   '8082'
[2024-02-08 10:33:19.613]: read_slab_count:        '500'
[2024-02-08 10:33:19.613]: reconns:                '2'
[2024-02-08 10:33:19.613]: sleep:                  '10'
[2024-02-08 10:33:19.613]: threads:                '2'
[2024-02-08 10:33:19.613]: tls:                    'true'
----------------------------------------------------------------------------------------------------
[2024-02-08 10:33:19.614]: (INFO) Client started in 7 ms.
[2024-02-08 10:33:19.663]: (INFO) sleeping for 10 ms.
[2024-02-08 10:33:19.663]: (INFO) sleeping for 10 ms.
[2024-02-08 10:33:19.723]: (INFO) sleeping for 10 ms.
[2024-02-08 10:33:19.723]: (INFO) sleeping for 10 ms.
----------------------------------------------------------------------------------------------------
[2024-02-08 10:33:19.733]: (INFO) Perf test completed successfully!
[2024-02-08 10:33:19.733]: (INFO) total_messages=[32],elapsed_time=[0.13s]
[2024-02-08 10:33:19.733]: (INFO) messages_per_second=[253],average_latency=[23006.15µs]
----------------------------------------------------------------------------------------------------
Latency Histogram
----------------------------------------------------------------------------------------------------
[100µs   - 110µs  ]======> 2 (6.25%)
[110µs   - 120µs  ]=========> 3 (9.38%)
[120µs   - 130µs  ]======> 2 (6.25%)
[140µs   - 150µs  ]===> 1 (3.12%)
[200µs   - 210µs  ]===> 1 (3.12%)
[220µs   - 230µs  ]===> 1 (3.12%)
[240µs   - 250µs  ]===> 1 (3.12%)
[250µs   - 260µs  ]===> 1 (3.12%)
[270µs   - 280µs  ]===> 1 (3.12%)
[280µs   - 290µs  ]======> 2 (6.25%)
[310µs   - 320µs  ]===> 1 (3.12%)
[44290µs - 44300µs]===> 1 (3.12%)
[44350µs - 44360µs]===> 1 (3.12%)
[44950µs - 44960µs]===> 1 (3.12%)
[44970µs - 44980µs]===> 1 (3.12%)
[45070µs - 45080µs]===> 1 (3.12%)
[45080µs - 45090µs]===> 1 (3.12%)
[45140µs - 45150µs]===> 1 (3.12%)
[45150µs - 45160µs]===> 1 (3.12%)
[45660µs - 45670µs]===> 1 (3.12%)
[45690µs - 45700µs]===> 1 (3.12%)
[45740µs - 45750µs]===> 1 (3.12%)
[47270µs - 47280µs]===> 1 (3.12%)
[47280µs - 47290µs]===> 1 (3.12%)
[47320µs - 47330µs]===> 1 (3.12%)
[47350µs - 47360µs]===> 1 (3.12%)
[47750µs - 47760µs]===> 1 (3.12%)
----------------------------------------------------------------------------------------------------
```

To build evh_perf, ensure you are in <project_subdirectory>/etc/evh_perf and then execute:

```text
$ cargo build --release
```

To run evh_perf:

```text
$ ./target/release/evh_perf --help
```

This will list the options.

# Performance

Here is the output of a run of the evh_perf tool on linux:

```
$ ./target/release/evh_perf -c -i 10 -t 20 --count 10000 --reconns 10
[2024-04-02 20:56:07.133]: evh_perf Client/0.0.3-beta.1
----------------------------------------------------------------------------------------------------
[2024-04-02 20:56:07.133]: clients:                '1'
[2024-04-02 20:56:07.133]: count:                  '10,000'
[2024-04-02 20:56:07.133]: debug:                  'false'
[2024-04-02 20:56:07.133]: histo:                  'false'
[2024-04-02 20:56:07.133]: histo_delta_micros:     '10'
[2024-04-02 20:56:07.133]: host:                   '127.0.0.1'
[2024-04-02 20:56:07.133]: iterations:             '10'
[2024-04-02 20:56:07.133]: max:                    '10'
[2024-04-02 20:56:07.133]: max_handles_per_thread: '300'
[2024-04-02 20:56:07.133]: min:                    '3'
[2024-04-02 20:56:07.133]: port:                   '8081'
[2024-04-02 20:56:07.133]: read_slab_count:        '20'
[2024-04-02 20:56:07.133]: reconns:                '10'
[2024-04-02 20:56:07.133]: sleep:                  '0'
[2024-04-02 20:56:07.133]: stats:                  'false'
[2024-04-02 20:56:07.133]: threads:                '20'
----------------------------------------------------------------------------------------------------
[2024-04-02 20:56:07.136]: (INFO)  Client started in 9 ms.
----------------------------------------------------------------------------------------------------
[2024-04-02 20:56:10.137]: (INFO)  8,011,446 of 20,000,000 messages received. [40.06% complete]
[2024-04-02 20:56:10.137]: (INFO)  incremental_messages=[8,011,446],elapsed_time=[3.00s]
[2024-04-02 20:56:10.137]: (INFO)  incremental_mps=[2,670,482],incremental_avg_latency=[47818.34µs]
[2024-04-02 20:56:10.137]: (INFO)  total_messages=[8,011,446],elapsed_time=[3.01s]
[2024-04-02 20:56:10.137]: (INFO)  total_mps=[2,661,594],total_avg_latency=[47818.34µs]
----------------------------------------------------------------------------------------------------
[2024-04-02 20:56:13.137]: (INFO)  16,036,291 of 20,000,000 messages received. [80.18% complete]
[2024-04-02 20:56:13.137]: (INFO)  incremental_messages=[8,024,845],elapsed_time=[3.00s]
[2024-04-02 20:56:13.137]: (INFO)  incremental_mps=[2,674,948],incremental_avg_latency=[41120.82µs]
[2024-04-02 20:56:13.137]: (INFO)  total_messages=[16,036,291],elapsed_time=[6.01s]
[2024-04-02 20:56:13.137]: (INFO)  total_mps=[2,668,182],total_avg_latency=[44466.78µs]
----------------------------------------------------------------------------------------------------
[2024-04-02 20:56:14.979]: (INFO)  Perf test completed successfully!
[2024-04-02 20:56:14.979]: (INFO)  total_messages=[20,000,000],elapsed_time=[7.85s]
[2024-04-02 20:56:14.979]: (INFO)  messages_per_second=[2,546,896],average_latency=[42679.07µs]
```

As seen above, the througput of this run was over 2.5 million messages per second. The latency, was around 42 ms per request. With less throughput the latency goes down.
On a 16 GB Ram Linux server, we tested over 1 million connections using the following command:

```
# ./target/release/evh_perf --connect --host 127.0.0.1 --count 15000
```

Due to limitations on Linux interfaces, we had to create multiple loop back interfaces to connect to. However, this is only a client side limitation and the server will handle any number of connections on it's external interface. There is a limitation with the number of file desciptors so that must be set high enough, but with some configuration that can be done.

Even after connecting 1 million clients, we were able to acheive high througput with the client test of well over 2 million request per second.

The eventhandler was using around 1.7 GB of RAM so, with 17 GB of RAM (and some additional RAM for the client connections), 10 million connections should be acheivable. That will be a test to be done on another day though.

