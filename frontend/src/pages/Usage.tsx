import { Suspense, useState, useEffect } from "react";
import { useLocation } from "react-router-dom";
import { useUsageStats } from "../hooks/useUsageStats.ts";

interface TimePeriodTabsProps {
    timePeriods: string[];
    selectedPeriod: string; // Allow any dynamic time period
    setSelectedPeriod: (period: string) => void; // Update function
}

const TimePeriodTabs: React.FC<TimePeriodTabsProps> = ({ timePeriods, selectedPeriod, setSelectedPeriod }) => {
    return (
      <div className="tab-container">
        {timePeriods.map((period) => (
            <span key={period} onClick={() => setSelectedPeriod(period)}
                className={`usage-tab ${selectedPeriod === period ? "usage-tab-active" : ""}`}>
                {period}
            </span>
        ))}
      </div>
    );
};

export default function Usage() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useUsageStats();

    type UsageKeys = {
        usage_stat_names: Record<string, string>;
        time_windows: Record<string, string>;
        select_accounts_by: Record<string, string>;
    };

    async function loadUsageKeys(): Promise<UsageKeys> {
        const response = await fetch("/usage_keys.json");
        if (!response.ok) {
            throw new Error(`Failed to load usage keys: ${response.statusText}`);
        }
        return response.json();
    }

    // Usage stats keys
    const [statsNames, setUsageStatNames] = useState<string[]>([]);
    const [timeWindows, setTimeWindows] = useState<string[]>([]);
    const [selectAccountsBy, setSelectAccountsBy] = useState<{ key: string; value: string }[]>([]);

    useEffect(() => {
        loadUsageKeys().then((keys) => {
            setUsageStatNames(Object.values(keys.usage_stat_names));
            setTimeWindows(Object.values(keys.time_windows));
            setSelectAccountsBy(Object.entries(keys.select_accounts_by).map(([key, value]) => ({ key, value })));
        }).catch(console.error);
    }, []);

    const [statPeriods, setStatPeriods] = useState<Record<string, string>>({});
    useEffect(() => {
        const defaultStatPeriods: Record<string, string> = {};
        statsNames.forEach((stat) => {
            // Default to first available time period
            defaultStatPeriods[stat] = timeWindows[0];
        });

        setStatPeriods(defaultStatPeriods);
    }, [data]); // Ensures this only runs when `data` updates


    return (
        <div className="usage-content">
            {/* Usage Monitor Page */}
            {pathname === "/usage" && (
            <div className="usage-container">
                {! data || error && <p className="error-text">Error loading data</p>}
                <Suspense fallback={<p className="loading-text">Loading...</p>}>
                    {isLoading ? (
                        <p className="loading-text">Loading...</p>
                    ) : (
                        <div>
                            <div className="usage-cards">
                                {statsNames.map((statName) => (
                                    <section key={statName} className="usage-section">
                                        <span className="usage-title">{statName}</span>
                                        <TimePeriodTabs
                                            timePeriods={timeWindows}
                                            selectedPeriod={statPeriods[statName]}
                                            setSelectedPeriod={(period) =>
                                                setStatPeriods((prev) => ({ ...prev, [statName]: period }))
                                            }
                                        />
                                        <div className="stat-value">
                                            {data?.stats[statName][statPeriods[statName]] ?? 0}
                                        </div>
                                    </section>
                                ))}
                            </div>
                            <div className="usage-cards">
                                {selectAccountsBy.map(({ key, value }) => {
                                const accounts = data?.selected_accounts[value] ?? []
                                return (
                                    <section key={value} className="accounts-section">
                                        <span className="usage-title">{value}</span>
                                        {accounts.length > 0 ? (
                                            <ul className="account-list">
                                                {accounts.map((account, index) => (
                                                    <li key={index} className="account-item">
                                                        <span className="account-address">{account.address}</span>
                                                        <span className="account-detail">
                                                            {key === "ACCOUNTS__RECENT"
                                                                ? `Deployed: ${new Date(account.creation_timestamp!).toLocaleString()}`
                                                                : `Gas Used: ${Number(account.gas_used!).toLocaleString()}`}
                                                        </span>
                                                    </li>
                                                ))}
                                            </ul>
                                        ) : (
                                            <p className="no-accounts">No accounts found.</p>
                                        )}
                                    </section>
                                    );
                                })}
                            </div>
                        </div>
                    )}
                </Suspense>
            </div>
            )}
        </div>
    );
}