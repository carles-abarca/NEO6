# NEO6 Project

NEO6 is a modular, extensible platform for legacy modernization and interoperability, focused on enabling seamless integration between modern applications and mainframe/COBOL systems. The project is organized into several subfolders, each with a specific role in the overall architecture.

## Folder Overview

- **agent-runtime/**: Core runtime and orchestration logic for NEO6 agents. Handles lifecycle, scheduling, and communication between agents and the platform.

- **aiops-agent/**: Components and logic for AIOps (Artificial Intelligence for IT Operations) within NEO6. Provides monitoring, anomaly detection, and automated remediation capabilities.

- **cobol-agent-core/**: Core libraries and utilities for COBOL agent integration. Facilitates communication and data mapping between COBOL programs and the NEO6 platform.

- **docs/**: Project documentation, architecture diagrams, and technical references for developers and users.

- **infra/**: Infrastructure-as-code, deployment scripts, and configuration for cloud/on-prem environments. Includes Terraform, Docker, and CI/CD resources.

- **neo6-protocols/**: Rust workspace containing protocol adapters and traits for all supported legacy and modern protocols (LU6.2, MQ, REST, TCP, JCA, etc). Includes:
  - `neo6-protocols-lib/`: Common protocol traits and helpers (e.g., `ProtocolHandler`).
  - `lu62/`, `mq/`, `rest/`, `tcp/`, `jca/`: Protocol-specific implementations.
  - `tests/`: Protocol-level tests.

- **neo6-proxy/**: Main proxy server for NEO6. Exposes REST, TCP, and MQ endpoints to modern and legacy clients, routing requests to the appropriate backend (COBOL, microservices, etc). Includes configuration, logging, metrics, and transaction mapping logic.

- **python-agent-core/**: Core libraries and runtime for Python-based agents. Used for scripting, orchestration, and integration tasks that require Python's flexibility.

- **vsr-tools/**: Utility tools and scripts for development, migration, and testing within the NEO6 ecosystem.

---

For more details on each component, see the README.md inside each folder or the documentation in `docs/`.
