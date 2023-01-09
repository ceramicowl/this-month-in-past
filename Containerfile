# # # # # # # # # # # # # # # # # # # #
# Builder
# # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # #
FROM docker.io/alpine:3 as builder

# Create an empty directory that will be used in the final image
RUN mkdir "/empty_dir"

# Install ssl certificates that will also be copied into the final image
RUN apk update && apk add --no-cache \
    ca-certificates bash file

# Copy all archs in to this container
RUN mkdir /work
WORKDIR /work
COPY target .
COPY stage-arch-bin.sh /work

# This will copy the cpu arch corresponding binary to /target/this-week-in-past
RUN bash stage-arch-bin.sh this-week-in-past

# # # # # # # # # # # # # # # # # # # #
# Run image
# # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # # #
FROM scratch

ENV USER "1337"
ENV RESOURCE_PATHS "/resources"
ENV DATA_FOLDER "/data"
ENV RUST_LOG "info"

# For performance reasons write data to docker volume instead of containers writeable fs layer
VOLUME $DATA_FOLDER

# Copy the empty directory as data and temp folder
COPY --chown=$USER:$USER --from=builder /empty_dir $DATA_FOLDER
COPY --chown=$USER:$USER --from=builder /empty_dir /tmp

# Copy the built application from the build image to the run-image
COPY --chown=$USER:$USER --from=builder /work/this-week-in-past /this-week-in-past

EXPOSE 8080
USER $USER

CMD ["/this-week-in-past"]
