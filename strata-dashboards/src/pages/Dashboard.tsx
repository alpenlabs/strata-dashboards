import { lazy, Suspense } from "react";
import { useNetworkStatus } from "../hooks/useNetworkStatus";

const StatusCard = lazy(() => import("../components/StatusCard"));

export default function Dashboard() {
    const { data, isLoading, error } = useNetworkStatus();

    return (
        <div className="dashboard">
            <div className="sidebar">
                {/* Logo Wrapper */}
                <a href="/" className="logoWrapper">
                    <div className="logoSvg">
                        <img src="/Strata_full logo_sand.png" alt="STRATA" />
                    </div>
                </a>
                {/* Menu */}
                <div className="menu">
                    <a href="/" className="menu-item active">
                        <div className="menu-icon">
                            <span className="menu-name">Network</span>
                        </div>
                    </a>
                    <a href="/bridge" className="menu-item active">
                        <div className="menu-icon">
                            <span className="menu-name">Bridge</span>
                        </div>
                    </a>
                    <a href="/usage" className="menu-item active">
                        <div className="menu-icon">
                            <span className="menu-name">Usage</span>
                        </div>
                    </a>
                </div>
            </div>
            <div className="content">
                <div className="status-container">
                    {error && <p className="error-text">Error loading data</p>}

                    <Suspense fallback={<p className="loading-text">Loading...</p>}>
                        {isLoading ? (
                            <p className="loading-text">Loading...</p>
                        ) : (
                            <div>

                                <div className="status-list">
                                    <StatusCard title="Batch producer status" status={data?.batch_producer ?? 'Unknown'} />
                                    <StatusCard title="RPC endpoint status" status={data?.rpc_endpoint ?? "Unknown"} />
                                    <StatusCard title="Bundler endpoint status" status={data?.bundler_endpoint ?? "Unknown"} />
                                </div>
                            </div>
                        )}
                    </Suspense>
                </div>
            </div>
        </div>
    );
}