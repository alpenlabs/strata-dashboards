import { lazy, Suspense } from "react";
import { Link, useLocation } from "react-router-dom";
import { useNetworkStatus } from "../hooks/useNetworkStatus";
import { usePaymasterWallets } from "../hooks/usePaymasterWallets";
import convertWeiToBtc from "../utils";

const StatusCard = lazy(() => import("../components/StatusCard"));
const BalanceCard = lazy(() => import("../components/BalanceCard"));
const Bridge = lazy(() => import("./Bridge"));
const Usage = lazy(() => import("./Usage"));

export default function Dashboard() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useNetworkStatus();
    const { data: wallets, isLoading: bal_isLoading, error: bal_error } = usePaymasterWallets();

    return (
        <div className="dashboard">
            <div className="sidebar">
                {/* Logo Wrapper */}
                <a href="/" className="logoWrapper">
                    <div className="logoSvg">
                        <img src="/Strata_full_logo_sand.png" alt="STRATA" />
                    </div>
                </a>
                {/* Menu */}
                <div className="menu">
                    <Link to="/" className={`menu-item ${pathname === "/" ? "active" : ""}`}>Network</Link>
                    <Link to="/bridge" className={`menu-item ${pathname === "/bridge" ? "active" : ""}`}>Bridge</Link>
                    <Link to="/usage" className={`menu-item ${pathname === "/usage" ? "active" : ""}`}>Usage</Link>
                </div>
            </div>

            <div className="content">
                {/* Network Monitor Page */}
                {pathname === "/" && (
                    <div className="status-container">
                        {error && <p className="error-text">Error loading data</p>}

                        <Suspense fallback={<p className="loading-text">Loading...</p>}>
                            {isLoading ? (
                                <p className="loading-text">Loading...</p>
                            ) : (
                                <div className="status-list">
                                    <StatusCard title="Batch producer status" status={data?.batch_producer ?? "Unknown"} />
                                    <StatusCard title="RPC endpoint status" status={data?.rpc_endpoint ?? "Unknown"} />
                                    <StatusCard title="Bundler endpoint status" status={data?.bundler_endpoint ?? "Unknown"} />
                                </div>
                            )}
                        </Suspense>
                    </div>
                )}

                {/* Paymaster Wallets Section */}
                {pathname === "/" && (
                    <div className="paymaster-container">
                        {bal_error && <p className="error-text">Error loading Paymaster Wallets</p>}

                        <Suspense fallback={<p className="loading-text">Loading paymaster balances...</p>}>
                            {bal_isLoading ? (
                                <p className="loading-text">Loading paymaster wallets...</p>
                            ) : wallets && wallets.deposit && wallets.validating ? (
                                <div className="paymaster-list">
                                    <div className="paymaster-item">
                                        <BalanceCard title="Deposit paymaster wallet" balance={convertWeiToBtc(wallets.deposit.balance)} />
                                    </div>
                                    <div className="paymaster-item">
                                        <BalanceCard title="Validating paymaster wallet" balance={convertWeiToBtc(wallets.validating.balance)} />
                                    </div>
                                </div>
                            ) : (
                                <p className="error-text">No Paymaster Data Available</p>
                            )}
                        </Suspense>
                    </div>
                )}

                {/* Bridge Page Content */}
                {pathname === "/bridge" && (
                    <div className="bridge-content">
                        <Bridge></Bridge>
                    </div>
                )}

                {/* Usage Page Content */}
                {pathname === "/usage" && (
                    <Usage></Usage>
                )}
            </div>
        </div>
    );
}