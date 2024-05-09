# Description
This project aims to allow users to self-host their own music server, with all the same content as Spotify, but instead using YouTube music as a data source.
The API allows for music to be downloaded via Apple Shortcuts, for which I have written a script. Upon receiving a download request, the API will spawn new downloader pods as Kubernetes jobs, constrained by the number of worker threads and the size of each worker that is provided as environment variables.
It is meant to be deployed on a Kubernetes cluster, and the API does not currently support running without Kubernetes. However, the Docker image for the downloader can theoretically be run standalone, provided the correct environment variables are passed in.

# Usage
You can choose to build the docker images yourself, or use the ones I am hosting at `docker.prayujt.com`. If you build them yourself, you will still need to self-host a docker registry in order for Kubernetes to be able to use your image.

## Setup
I have provided a sample Kubernetes configuration in this repository under `config/`. 
In this directory, you will need to edit the `distributed-streaming.yaml` file so that it uses the correct Docker image (the one hosted by myself is currently set), and the correct PVCs and secrets. 
The deployment given uses `distributed-streaming-secrets` as a secrets ref. This just passes in the following environment variables, which you can create in a secrets file or pass in as an env in the deployment directly. All of the ones without default values are required to run the image.
- MUSIC_STORAGE_PVC: String (the PVC for the volume with your music)
- SPOTIFY_CLIENT_ID: String
- SPOTIFY_CLIENT_SECRET: String
- WORKER_SIZE: Int (defaults to 5)
- NUM_WORKERS: Int (defaults to 8)
This is all that you need to run the API. With the secrets passed in, you can run
```
kubectl apply -f distributed-streaming.yaml
```
to start up the pod. The yaml specification will also create permissions for the pod to spin up new Kubernetes jobs, which are needed for the distributed downloading.

**Note: The downloader jobs that are spun up will use the same PVC that you passed in as an env, so make sure that it has `ReadWriteMany` permissions so that multiple jobs can use it simultaneously.**
