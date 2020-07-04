# Copyright 2020 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import logging
import os
import subprocess
from concurrent import futures

import grpc

import observer_service_pb2
import observer_service_pb2_grpc


class CoverageObserverService(observer_service_pb2_grpc.ObserverServiceServicer):
    def __init__(self):
        self.struct_graph = None
        self.node_map = None
        self.coverage = 0

    def CreateFuzzer(self, req, ctx):
        if req.fuzzer_id == 0:
            self.struct_graph = req.structure_graph
            self.node_map = [False] * len(self.struct_graph.nodes)

        return observer_service_pb2.CreateFuzzerResponse()

    def UpdateFeatures(self, req, ctx):
        has_update = False
        for bit_counter in req.bit_counters:
            if not self.node_map[bit_counter.node_index]:
                self.node_map[bit_counter.node_index] = True
                self.coverage += 1
                has_update = True

        if has_update:
            print(f'{self.coverage} / {len(self.struct_graph.nodes)}')

        return observer_service_pb2.UpdateFeaturesResponse()


def start_server():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    observer_service_pb2_grpc.add_ObserverServiceServicer_to_server(
        CoverageObserverService(), server)
    observer_addr = '[::1]:2573'
    server.add_insecure_port(observer_addr)
    server.start()

    observer_proxy_path = os.path.join(os.path.dirname(__file__), '..', '..',
                                       'target', 'release', 'observer_proxy')
    collector_addr = '[::1]:2501'
    with subprocess.Popen([observer_proxy_path, '--listen_addr',
                           f'{collector_addr}', '--observer_url', f'http://{observer_addr}']):
        server.wait_for_termination()


if __name__ == '__main__':
    logging.basicConfig()
    start_server()
