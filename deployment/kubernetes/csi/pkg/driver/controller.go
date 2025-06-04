package driver

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strconv"

	"github.com/container-storage-interface/spec/lib/go/csi"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"k8s.io/klog/v2"
)

const (
	// VexFS volume parameters
	VexFSVolumeSize = "vexfs.volume.size"
	VexFSVectorDim  = "vexfs.vector.dimension"
	VexFSIndexType  = "vexfs.index.type"
)

// CreateVolume creates a new VexFS volume
func (d *Driver) CreateVolume(ctx context.Context, req *csi.CreateVolumeRequest) (*csi.CreateVolumeResponse, error) {
	if len(req.GetName()) == 0 {
		return nil, status.Error(codes.InvalidArgument, "Volume name missing in request")
	}

	if req.GetVolumeCapabilities() == nil {
		return nil, status.Error(codes.InvalidArgument, "Volume capabilities missing in request")
	}

	// Validate volume capabilities
	if err := d.ValidateVolumeCapabilities(req.GetVolumeCapabilities()); err != nil {
		return nil, status.Error(codes.InvalidArgument, err.Error())
	}

	volumeID := req.GetName()
	size := int64(1 * 1024 * 1024 * 1024) // Default 1GB

	// Parse capacity range
	if req.GetCapacityRange() != nil {
		if req.GetCapacityRange().GetRequiredBytes() > 0 {
			size = req.GetCapacityRange().GetRequiredBytes()
		}
	}

	// Parse VexFS-specific parameters
	parameters := req.GetParameters()
	if parameters != nil {
		if sizeStr, ok := parameters[VexFSVolumeSize]; ok {
			if parsedSize, err := strconv.ParseInt(sizeStr, 10, 64); err == nil {
				size = parsedSize
			}
		}
	}

	klog.Infof("Creating VexFS volume %s with size %d bytes", volumeID, size)

	// Create volume directory structure
	volumePath := filepath.Join("/var/lib/vexfs/volumes", volumeID)
	if err := os.MkdirAll(volumePath, 0755); err != nil {
		return nil, status.Errorf(codes.Internal, "Failed to create volume directory: %v", err)
	}

	// Create VexFS metadata
	metadataPath := filepath.Join(volumePath, "vexfs.meta")
	metadata := fmt.Sprintf("volume_id=%s\nsize=%d\ncreated_by=vexfs-csi\n", volumeID, size)
	if err := os.WriteFile(metadataPath, []byte(metadata), 0644); err != nil {
		return nil, status.Errorf(codes.Internal, "Failed to create volume metadata: %v", err)
	}

	volume := &csi.Volume{
		VolumeId:      volumeID,
		CapacityBytes: size,
		VolumeContext: parameters,
	}

	return &csi.CreateVolumeResponse{Volume: volume}, nil
}

// DeleteVolume deletes a VexFS volume
func (d *Driver) DeleteVolume(ctx context.Context, req *csi.DeleteVolumeRequest) (*csi.DeleteVolumeResponse, error) {
	volumeID := req.GetVolumeId()
	if len(volumeID) == 0 {
		return nil, status.Error(codes.InvalidArgument, "Volume ID missing in request")
	}

	klog.Infof("Deleting VexFS volume %s", volumeID)

	volumePath := filepath.Join("/var/lib/vexfs/volumes", volumeID)
	if err := os.RemoveAll(volumePath); err != nil && !os.IsNotExist(err) {
		return nil, status.Errorf(codes.Internal, "Failed to delete volume: %v", err)
	}

	return &csi.DeleteVolumeResponse{}, nil
}

// ControllerPublishVolume attaches a volume to a node
func (d *Driver) ControllerPublishVolume(ctx context.Context, req *csi.ControllerPublishVolumeRequest) (*csi.ControllerPublishVolumeResponse, error) {
	volumeID := req.GetVolumeId()
	nodeID := req.GetNodeId()

	if len(volumeID) == 0 {
		return nil, status.Error(codes.InvalidArgument, "Volume ID missing in request")
	}

	if len(nodeID) == 0 {
		return nil, status.Error(codes.InvalidArgument, "Node ID missing in request")
	}

	klog.Infof("Publishing VexFS volume %s to node %s", volumeID, nodeID)

	return &csi.ControllerPublishVolumeResponse{}, nil
}

// ControllerUnpublishVolume detaches a volume from a node
func (d *Driver) ControllerUnpublishVolume(ctx context.Context, req *csi.ControllerUnpublishVolumeRequest) (*csi.ControllerUnpublishVolumeResponse, error) {
	volumeID := req.GetVolumeId()
	nodeID := req.GetNodeId()

	if len(volumeID) == 0 {
		return nil, status.Error(codes.InvalidArgument, "Volume ID missing in request")
	}

	klog.Infof("Unpublishing VexFS volume %s from node %s", volumeID, nodeID)

	return &csi.ControllerUnpublishVolumeResponse{}, nil
}

// ValidateVolumeCapabilities validates volume capabilities
func (d *Driver) ValidateVolumeCapabilities(ctx context.Context, req *csi.ValidateVolumeCapabilitiesRequest) (*csi.ValidateVolumeCapabilitiesResponse, error) {
	volumeID := req.GetVolumeId()
	if len(volumeID) == 0 {
		return nil, status.Error(codes.InvalidArgument, "Volume ID missing in request")
	}

	volumeCaps := req.GetVolumeCapabilities()
	if volumeCaps == nil {
		return nil, status.Error(codes.InvalidArgument, "Volume capabilities missing in request")
	}

	var confirmed *csi.ValidateVolumeCapabilitiesResponse_Confirmed
	if err := d.ValidateVolumeCapabilities(volumeCaps); err == nil {
		confirmed = &csi.ValidateVolumeCapabilitiesResponse_Confirmed{VolumeCapabilities: volumeCaps}
	}

	return &csi.ValidateVolumeCapabilitiesResponse{
		Confirmed: confirmed,
	}, nil
}

// ListVolumes lists all volumes
func (d *Driver) ListVolumes(ctx context.Context, req *csi.ListVolumesRequest) (*csi.ListVolumesResponse, error) {
	klog.V(5).Infof("ListVolumes called")

	volumesDir := "/var/lib/vexfs/volumes"
	entries, err := os.ReadDir(volumesDir)
	if err != nil {
		if os.IsNotExist(err) {
			return &csi.ListVolumesResponse{}, nil
		}
		return nil, status.Errorf(codes.Internal, "Failed to list volumes: %v", err)
	}

	var volumes []*csi.ListVolumesResponse_Entry
	for _, entry := range entries {
		if entry.IsDir() {
			volumes = append(volumes, &csi.ListVolumesResponse_Entry{
				Volume: &csi.Volume{
					VolumeId: entry.Name(),
				},
			})
		}
	}

	return &csi.ListVolumesResponse{
		Entries: volumes,
	}, nil
}

// GetCapacity returns the capacity of the storage pool
func (d *Driver) GetCapacity(ctx context.Context, req *csi.GetCapacityRequest) (*csi.GetCapacityResponse, error) {
	klog.V(5).Infof("GetCapacity called")
	return &csi.GetCapacityResponse{}, nil
}

// ControllerGetCapabilities returns the capabilities of the controller service
func (d *Driver) ControllerGetCapabilities(ctx context.Context, req *csi.ControllerGetCapabilitiesRequest) (*csi.ControllerGetCapabilitiesResponse, error) {
	klog.V(5).Infof("ControllerGetCapabilities called")
	return &csi.ControllerGetCapabilitiesResponse{
		Capabilities: d.cscap,
	}, nil
}

// CreateSnapshot creates a snapshot
func (d *Driver) CreateSnapshot(ctx context.Context, req *csi.CreateSnapshotRequest) (*csi.CreateSnapshotResponse, error) {
	return nil, status.Error(codes.Unimplemented, "CreateSnapshot not implemented")
}

// DeleteSnapshot deletes a snapshot
func (d *Driver) DeleteSnapshot(ctx context.Context, req *csi.DeleteSnapshotRequest) (*csi.DeleteSnapshotResponse, error) {
	return nil, status.Error(codes.Unimplemented, "DeleteSnapshot not implemented")
}

// ListSnapshots lists snapshots
func (d *Driver) ListSnapshots(ctx context.Context, req *csi.ListSnapshotsRequest) (*csi.ListSnapshotsResponse, error) {
	return nil, status.Error(codes.Unimplemented, "ListSnapshots not implemented")
}

// ControllerExpandVolume expands a volume
func (d *Driver) ControllerExpandVolume(ctx context.Context, req *csi.ControllerExpandVolumeRequest) (*csi.ControllerExpandVolumeResponse, error) {
	return nil, status.Error(codes.Unimplemented, "ControllerExpandVolume not implemented")
}

// ControllerGetVolume gets volume information
func (d *Driver) ControllerGetVolume(ctx context.Context, req *csi.ControllerGetVolumeRequest) (*csi.ControllerGetVolumeResponse, error) {
	return nil, status.Error(codes.Unimplemented, "ControllerGetVolume not implemented")
}
