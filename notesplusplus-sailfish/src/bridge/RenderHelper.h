/* RenderHelper.h — AsciiDoc element preview rendering.
 *
 * Renders hardcoded AsciiDoc snippets to Qt HTML for the element picker UI.
 * Self-contained: only needs FFI + BridgeContext.
 */

#ifndef RENDERHELPER_H
#define RENDERHELPER_H

#include <QString>
#include "BridgeContext.h"

class RenderHelper
{
public:
    explicit RenderHelper(const BridgeContext &ctx);

    QString render_element_previews();

private:
    const BridgeContext &m_ctx;
};

#endif /* RENDERHELPER_H */
