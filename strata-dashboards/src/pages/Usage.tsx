import { Suspense, useState } from "react";
import { useLocation } from "react-router-dom";
import { useUsageStats } from "../hooks/useUsageStats.ts";
  
interface TimePeriodTabsProps {
    selectedPeriod: string; // Allow any dynamic time period
    setSelectedPeriod: (period: string) => void; // Update function
}

const TimePeriodTabs: React.FC<TimePeriodTabsProps> = ({ selectedPeriod, setSelectedPeriod }) => {
    return (
      <div className="tab-container">
        {(["24h", "30d", "YTD"] as const).map((period) => (
          <span
            key={period}
            onClick={() => setSelectedPeriod(period)}
            className={`usage-tab ${selectedPeriod === period ? "usage-tab-active" : ""}`}
          >
            {period}
          </span>
        ))}
      </div>
    );
};

export default function Usage() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useUsageStats();
    if (isLoading) return <p className="loading-text">Loading...</p>
    if (error || !data) return <p className="error-text">Error loading data</p>
    
    const [selectedPeriods, setSelectedPeriods] = useState<Record<string, string>>(() => {
        // Provide an initial state to avoid undefined errors
        if (!data || !Object.keys(data.stats).length) return {};
      
        const statNames = Object.keys(data.stats);
        const firstStatName = statNames[0];
        const timePeriods = firstStatName ? Object.keys(data.stats[firstStatName]) : [];
        const defaultSelectedPeriods: Record<string, string> = {};
        statNames.forEach((stat) => {
          defaultSelectedPeriods[stat] = timePeriods[0]; // Default to first available time period
        });
      
        return defaultSelectedPeriods;
      });

    return (
        <div className="usage-content">
            {/* Usage Monitor Page */}
            {pathname === "/usage" && (
                <div className="usage-container">
                    <div className="usage-cards">
                        {Object.keys(data.stats).map((statName) => (
                            <section key={statName} className="usage-section">
                                <text className="usage-title">{statName}</text> {/* Format title */}
                                <TimePeriodTabs
                                    selectedPeriod={selectedPeriods[statName]}
                                    setSelectedPeriod={(period) =>
                                        setSelectedPeriods((prev) => ({ ...prev, [statName]: period }))
                                    }
                                />
                                <div className="stat-value">
                                    {data.stats[statName][selectedPeriods[statName]]}
                                </div>
                            </section>
                        ))}
                        {/* <section className="usage-cards">
                            <UsageCard title="User Ops Count" stat={data?.user_ops_count ?? "Unknown"} />
                            <UsageCard title="Total Gas Used" stat={data?.total_gas_used ?? "Unknown"} />
                            <UsageCard title="Unique Active Acounts" stat={data?.unique_active_accounts ?? "Unknown"} />
                        </section> */}
                    </div>
                </div>
            )}
        </div>
    );
}