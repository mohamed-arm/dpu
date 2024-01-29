
VERSION = v1.0
CONTAINER_NAME = dpu_build

DOCKER_BUILD_CMD = DOCKER_BUILDKIT=1 docker build
DOCKER_CREATE_CMD = docker create
DOCKER_BUILD_ARGS = --build-arg ARCH=aarch64 --build-arg DOCKER_ARCH=arm64v8/ --build-arg VERSION=$(VERSION) 


build: 
	$(DOCKER_BUILD_CMD) $(DOCKER_BUILD_ARGS) -t $(CONTAINER_NAME):$(VERSION)  -f Dockerfile .
 
exec:
	docker run -ti -v `pwd`/..:/work $(CONTAINER_NAME):v1.0
