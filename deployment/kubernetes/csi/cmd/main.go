package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/vexfs/vexfs/deployment/kubernetes/csi/pkg/driver"
	"k8s.io/klog/v2"
)

var (
	version    = "1.0.0"
	driverName = "vexfs.csi.k8s.io"
)

func main() {
	var (
		endpoint   = flag.String("endpoint", "unix://tmp/csi.sock", "CSI endpoint")
		nodeID     = flag.String("nodeid", "", "node id")
		showVer    = flag.Bool("version", false, "Show version")
		maxVolumes = flag.Int64("max-volumes-per-node", 0, "limit of volumes per node")
	)
	klog.InitFlags(nil)
	flag.Parse()

	if *showVer {
		fmt.Printf("VexFS CSI Driver\n")
		fmt.Printf("Version: %s\n", version)
		fmt.Printf("Driver Name: %s\n", driverName)
		os.Exit(0)
	}

	if *nodeID == "" {
		klog.Error("nodeid must be provided")
		os.Exit(1)
	}

	klog.Infof("Starting VexFS CSI Driver, version: %s, driver: %s", version, driverName)

	d, err := driver.NewDriver(driverName, version, *nodeID, *maxVolumes)
	if err != nil {
		klog.Fatalf("Failed to initialize driver: %v", err)
	}

	if err := d.Run(*endpoint); err != nil {
		klog.Fatalf("Failed to run driver: %v", err)
	}
}
