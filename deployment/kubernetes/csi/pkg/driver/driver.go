package driver

import (
	"net"
	"net/url"
	"os"
	"path"
	"path/filepath"

	"github.com/container-storage-interface/spec/lib/go/csi"
	"google.golang.org/grpc"
	"k8s.io/klog/v2"
)

const (
	DefaultDriverName = "vexfs.csi.k8s.io"
	DefaultVersion    = "1.0.0"
)

// Driver implements the CSI specification
type Driver struct {
	name       string
	version    string
	nodeID     string
	maxVolumes int64

	srv   *grpc.Server
	cap   []*csi.VolumeCapability_AccessMode
	cscap []*csi.ControllerServiceCapability
	nscap []*csi.NodeServiceCapability
}

// NewDriver creates a new VexFS CSI driver
func NewDriver(driverName, version, nodeID string, maxVolumes int64) (*Driver, error) {
	if driverName == "" {
		driverName = DefaultDriverName
	}
	if version == "" {
		version = DefaultVersion
	}

	klog.Infof("Driver: %v version: %v", driverName, version)

	d := &Driver{
		name:       driverName,
		version:    version,
		nodeID:     nodeID,
		maxVolumes: maxVolumes,
	}

	d.AddVolumeCapabilityAccessModes([]csi.VolumeCapability_AccessMode_Mode{
		csi.VolumeCapability_AccessMode_SINGLE_NODE_WRITER,
		csi.VolumeCapability_AccessMode_SINGLE_NODE_READER_ONLY,
		csi.VolumeCapability_AccessMode_MULTI_NODE_READER_ONLY,
	})

	d.AddControllerServiceCapabilities([]csi.ControllerServiceCapability_RPC_Type{
		csi.ControllerServiceCapability_RPC_CREATE_DELETE_VOLUME,
		csi.ControllerServiceCapability_RPC_PUBLISH_UNPUBLISH_VOLUME,
		csi.ControllerServiceCapability_RPC_CREATE_DELETE_SNAPSHOT,
		csi.ControllerServiceCapability_RPC_LIST_SNAPSHOTS,
		csi.ControllerServiceCapability_RPC_EXPAND_VOLUME,
	})

	d.AddNodeServiceCapabilities([]csi.NodeServiceCapability_RPC_Type{
		csi.NodeServiceCapability_RPC_STAGE_UNSTAGE_VOLUME,
		csi.NodeServiceCapability_RPC_EXPAND_VOLUME,
		csi.NodeServiceCapability_RPC_GET_VOLUME_STATS,
	})

	return d, nil
}

// Run starts the CSI driver
func (d *Driver) Run(endpoint string) error {
	u, err := url.Parse(endpoint)
	if err != nil {
		return err
	}

	addr := path.Join(u.Host, filepath.FromSlash(u.Path))
	if u.Host == "" {
		addr = filepath.FromSlash(u.Path)
	}

	// Remove existing socket file
	if err := os.Remove(addr); err != nil && !os.IsNotExist(err) {
		return err
	}

	listener, err := net.Listen(u.Scheme, addr)
	if err != nil {
		return err
	}

	d.srv = grpc.NewServer()
	csi.RegisterIdentityServer(d.srv, d)
	csi.RegisterControllerServer(d.srv, d)
	csi.RegisterNodeServer(d.srv, d)

	klog.Infof("Listening for connections on address: %#v", listener.Addr())
	return d.srv.Serve(listener)
}

// Stop stops the CSI driver
func (d *Driver) Stop() {
	klog.Infof("Stopping server")
	d.srv.Stop()
}

// AddVolumeCapabilityAccessModes adds volume capability access modes
func (d *Driver) AddVolumeCapabilityAccessModes(vc []csi.VolumeCapability_AccessMode_Mode) {
	var vca []*csi.VolumeCapability_AccessMode
	for _, c := range vc {
		klog.Infof("Enabling volume access mode: %v", c.String())
		vca = append(vca, &csi.VolumeCapability_AccessMode{Mode: c})
	}
	d.cap = vca
}

// AddControllerServiceCapabilities adds controller service capabilities
func (d *Driver) AddControllerServiceCapabilities(cl []csi.ControllerServiceCapability_RPC_Type) {
	var csc []*csi.ControllerServiceCapability
	for _, c := range cl {
		klog.Infof("Enabling controller service capability: %v", c.String())
		csc = append(csc, &csi.ControllerServiceCapability{
			Type: &csi.ControllerServiceCapability_Rpc{
				Rpc: &csi.ControllerServiceCapability_RPC{
					Type: c,
				},
			},
		})
	}
	d.cscap = csc
}

// AddNodeServiceCapabilities adds node service capabilities
func (d *Driver) AddNodeServiceCapabilities(nl []csi.NodeServiceCapability_RPC_Type) {
	var nsc []*csi.NodeServiceCapability
	for _, n := range nl {
		klog.Infof("Enabling node service capability: %v", n.String())
		nsc = append(nsc, &csi.NodeServiceCapability{
			Type: &csi.NodeServiceCapability_Rpc{
				Rpc: &csi.NodeServiceCapability_RPC{
					Type: n,
				},
			},
		})
	}
	d.nscap = nsc
}

// ValidateVolumeCapabilities validates volume capabilities
func (d *Driver) ValidateVolumeCapabilities(volumeCaps []*csi.VolumeCapability) error {
	for _, volCap := range volumeCaps {
		if volCap.GetAccessMode() == nil {
			return nil
		}
		supported := false
		for _, c := range d.cap {
			if c.GetMode() == volCap.GetAccessMode().GetMode() {
				supported = true
				break
			}
		}
		if !supported {
			return nil
		}
	}
	return nil
}
