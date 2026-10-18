# {{project_name}} container runtime

This runtime runs `{{build_command}}` in a build stage, copies `{{output_dir}}`
to the production image, and starts the application with `{{start_command}}`.
The Compose health check probes `{{health_path}}`.

The commands are project-specific inputs and are recorded in the generated
Dockerfile; review them before promoting the project to deployable.
