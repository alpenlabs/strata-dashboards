import { Suspense } from "react";
import { useLocation } from "react-router-dom";
import { useBridgeStatus, OperatorStatus } from "../hooks/useBridgeStatus";

export default function Bridge() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useBridgeStatus();
    console.log(data);

    return (
        <div className="bridge-content">
            {/* Bridge Status Page */}
            {pathname === "/bridge" && (
                <div className="bridge-container">
                    {!data || error ? <p className="error-text">Error loading data</p> : null}
                    <Suspense fallback={<p className="loading-text">Loading...</p>}>
                        {isLoading ? (
                            <p className="loading-text">Loading...</p>
                        ) : (
                            <div className="operators-section">
                                <span className="operators-title">Bridge operator status</span>
                                { data && data.operators ? (
                                    <div className="table-wrapper">
                                        <table className="operators-table">
                                            <tbody>
                                                {data.operators.map((operator: OperatorStatus, index: number) => (
                                                    <tr key={index} className="operators-row">
                                                        <td className="operator-status">{operator.operator_id}</td>
                                                        <td className="operator-status">{operator.operator_address}</td>
                                                        <td className={`operator-status ${operator.status.toLowerCase()}`}>
                                                            {operator.status}
                                                        </td>
                                                    </tr>
                                                ))}
                                            </tbody>
                                        </table>
                                    </div>
                                ) : (
                                    <p className="no-items">No bridge operators found.</p>
                                )}
                            </div>
                        )}
                    </Suspense>
                </div>
            )}
        </div>
    );
}