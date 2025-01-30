import { Suspense, useState } from "react";
import { useLocation } from "react-router-dom";
import { useUsageStats } from "../hooks/useUsageStats.ts";
  
interface TimePeriodTabsProps {
    selectedPeriod: "24h" | "30d" | "YTD";
    setSelectedPeriod: React.Dispatch<React.SetStateAction<"24h" | "30d" | "YTD">>;
    statGroup: "ops" | "gas" | "accounts";
}

const TimePeriodTabs: React.FC<TimePeriodTabsProps> = ({ selectedPeriod, setSelectedPeriod, statGroup }) => {
    return (
      <div className={`tab-container ${statGroup}`}>
        {(["24h", "30d", "YTD"] as const).map((period) => (
          <button
            key={period}
            onClick={() => setSelectedPeriod(period)}
            className={`usage-button ${selectedPeriod === period ? `usage-button-active usage-${statGroup}-active` : ""}`}
          >
            {period}
          </button>
        ))}
      </div>
    );
};

export default function Usage() {
    const { pathname } = useLocation(); // Get current URL path
    const [selectedUserOps, setSelectedUserOps] = useState<"24h" | "30d" | "YTD">("24h");
    const [selectedGasUsed, setSelectedGasUsed] = useState<"24h" | "30d" | "YTD">("24h");
    const [selectedUniqueAccounts, setSelectedUniqueAccounts] = useState<"24h" | "30d" | "YTD">("24h");
    const { data, isLoading, error } = useUsageStats();
    if (!data) return null;

    return (
        <div className="usage-content">
            {/* Usage Monitor Page */}
            {pathname === "/usage" && (
                <div className="usage-container">
                    {error && <p className="error-text">Error loading data</p>}
                    <Suspense fallback={<p className="loading-text">Loading...</p>}>
                    {isLoading ? (
                        <p className="loading-text">Loading...</p>
                    ) : (
                        <div className="usage-cards">
                            <section className="usage-section">
                                <text className="usage-title">User ops</text>
                                <TimePeriodTabs
                                    selectedPeriod={selectedUserOps}
                                    setSelectedPeriod={setSelectedUserOps}
                                    statGroup="ops"
                                />
                                <div className="stat-value">
                                    {data.user_ops_count[selectedUserOps]}
                                </div>
                            </section>
                            <section className="usage-section">
                                <text className="usage-title">Gas used</text>
                                <TimePeriodTabs
                                    selectedPeriod={selectedGasUsed}
                                    setSelectedPeriod={setSelectedGasUsed}
                                    statGroup="gas"
                                />
                                <div className="stat-value">
                                    {data.total_gas_used[selectedGasUsed]}
                                </div>
                            </section>
                            <section className="usage-section">
                                <text className="usage-title">Unique active accounts</text>
                                <TimePeriodTabs
                                    selectedPeriod={selectedUniqueAccounts}
                                    setSelectedPeriod={setSelectedUniqueAccounts}
                                    statGroup="accounts"
                                />
                                <div className="stat-value">
                                    {data.unique_active_accounts[selectedUniqueAccounts]}
                                </div>
                            </section>
                            {/* <section className="usage-cards">
                                <UsageCard title="User Ops Count" stat={data?.user_ops_count ?? "Unknown"} />
                                <UsageCard title="Total Gas Used" stat={data?.total_gas_used ?? "Unknown"} />
                                <UsageCard title="Unique Active Acounts" stat={data?.unique_active_accounts ?? "Unknown"} />
                            </section> */}
                        </div>
                    )}
                    </Suspense>
                </div>
                )}
        </div>
    );
}