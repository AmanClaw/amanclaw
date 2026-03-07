# User Journey Improvements — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve the user journey from /start through approval to first interaction — eliminate dead ends, add inline admin approval buttons, and create a warm onboarding experience with "try me" examples.

**Architecture:** All changes in `bot.py`. No DB schema changes. Rewrite `handle_registration()` for better messaging, add inline approve/block buttons for admins, rewrite `cmd_start` to branch on onboarded state, add "try me" callback handlers, and track onboarded fact after first interaction.

**Tech Stack:** python-telegram-bot 21.7, existing Memory/Auth modules

---

### Task 1: Rewrite handle_registration() — Better Messages + Inline Admin Buttons

**Files:**
- Modify: `amanclaw/bot.py:196-248`

**Step 1: Rewrite handle_registration()**

Replace lines 196-248 with:

```python
async def handle_registration(update: Update, context: ContextTypes.DEFAULT_TYPE) -> bool:
    """Handle user registration flow. Returns True if user can proceed, False if blocked/pending."""
    user = update.effective_user
    user_id = str(user.id)
    state = auth.get_user_state(user_id, "telegram")

    if state == "admin" or state == "approved":
        return True

    if state == "blocked":
        await update.message.reply_text(
            "Sorry, your access has been denied. "
            "Contact the admin if you think this is a mistake."
        )
        return False

    if state == "pending":
        await update.message.reply_text(
            "Still waiting for admin approval. Hang tight — "
            "you'll get a message as soon as you're in!\n\n"
            "Send /start anytime to check your status."
        )
        return False

    # New user — register and notify admins
    memory.register_user(
        user_id=user_id,
        platform="telegram",
        username=user.username,
        first_name=user.first_name,
        last_name=user.last_name,
    )
    await update.message.reply_text(
        "Welcome to AmanClaw!\n\n"
        "I'm a smart AI assistant that can remember things about you, "
        "analyze photos, set reminders, and much more.\n\n"
        "Your registration has been sent to an admin for approval. "
        "You'll be notified as soon as you're approved — usually within minutes!\n\n"
        "Send /start anytime to check your status."
    )

    # Notify all admins with inline approve/block buttons
    admin_ids = config.get("admin_users", {}).get("telegram", [])
    name = escape_markdown(user.first_name or user.username or user_id)
    admin_keyboard = InlineKeyboardMarkup([
        [
            InlineKeyboardButton("Approve", callback_data=f"adm_approve_{user_id}"),
            InlineKeyboardButton("Block", callback_data=f"adm_block_{user_id}"),
        ]
    ])
    for admin_id in admin_ids:
        try:
            await context.bot.send_message(
                chat_id=int(admin_id),
                text=(
                    f"*New user registration:*\n\n"
                    f"Name: {name}\n"
                    f"Username: @{escape_markdown(user.username or 'none')}\n"
                    f"User ID: `{user_id}`"
                ),
                parse_mode=ParseMode.MARKDOWN,
                reply_markup=admin_keyboard,
            )
        except Exception as e:
            logger.error(f"Failed to notify admin {admin_id}: {e}")

    return False
```

**Step 2: Verify the edit compiles**

Run: `python -c "import amanclaw.bot"`
Expected: No import errors

**Step 3: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: improve registration messages + inline admin approve/block buttons"
```

---

### Task 2: Add Admin Inline Button Callbacks (approve/block)

**Files:**
- Modify: `amanclaw/bot.py` — inside `handle_callback()` (line ~796)

**Step 1: Add admin callback handlers**

In `handle_callback()`, add these branches BEFORE the existing `elif query.data == "skills":` line:

```python
    # --- Admin approval callbacks ---
    if query.data.startswith("adm_approve_"):
        admin_id = str(query.from_user.id)
        if not auth.is_admin(admin_id, "telegram"):
            await query.answer("Not authorized.", show_alert=True)
            return
        target_id = query.data.replace("adm_approve_", "")
        if memory.approve_user(target_id):
            await query.edit_message_text(
                query.message.text + "\n\n*Approved*",
                parse_mode=ParseMode.MARKDOWN,
            )
            # Send welcome to approved user
            await send_approval_welcome(context, target_id)
        else:
            await query.answer("User already processed.", show_alert=True)
        return

    if query.data.startswith("adm_block_"):
        admin_id = str(query.from_user.id)
        if not auth.is_admin(admin_id, "telegram"):
            await query.answer("Not authorized.", show_alert=True)
            return
        target_id = query.data.replace("adm_block_", "")
        if memory.block_user(target_id):
            await query.edit_message_text(
                query.message.text + "\n\n*Blocked*",
                parse_mode=ParseMode.MARKDOWN,
            )
        else:
            await query.answer("User already processed.", show_alert=True)
        return
```

**Step 2: Create `send_approval_welcome()` helper**

Add this function BEFORE `handle_callback()`:

```python
async def send_approval_welcome(context: ContextTypes.DEFAULT_TYPE, user_id: str):
    """Send the post-approval welcome message with try-me buttons."""
    welcome_keyboard = InlineKeyboardMarkup([
        [
            InlineKeyboardButton("Tell me your name", callback_data="try_name"),
            InlineKeyboardButton("Analyze a photo", callback_data="try_photo"),
        ],
        [
            InlineKeyboardButton("Set a reminder", callback_data="try_reminder"),
            InlineKeyboardButton("How do I teach you?", callback_data="try_teach"),
        ],
        [
            InlineKeyboardButton("Set language", callback_data="onboard_lang"),
        ],
    ])
    try:
        await context.bot.send_message(
            chat_id=int(user_id),
            text=(
                "You're approved! Welcome to AmanClaw.\n\n"
                "I'm your personal AI assistant. Here's what I can do:\n\n"
                "I remember things about you across conversations\n"
                "Send me a photo and I'll analyze it\n"
                "I can set reminders for you\n"
                "Teach me custom rules and I'll follow them\n\n"
                "Try one of these to get started, or just say hi!"
            ),
            reply_markup=welcome_keyboard,
        )
    except Exception as e:
        logger.error(f"Failed to send welcome to {user_id}: {e}")
```

**Step 3: Verify**

Run: `python -c "import amanclaw.bot"`
Expected: No import errors

**Step 4: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: add inline admin approve/block callbacks + approval welcome message"
```

---

### Task 3: Add "Try Me" Callback Handlers

**Files:**
- Modify: `amanclaw/bot.py` — inside `handle_callback()`

**Step 1: Add try-me callback handlers**

Add these branches inside `handle_callback()`, after the admin callbacks:

```python
    # --- Try-me onboarding callbacks ---
    if query.data == "try_name":
        await query.edit_message_text(
            "Just send me a message telling me your name!\n\n"
            "For example: \"My name is Sarah\" or \"Call me Alex\"\n\n"
            "I'll remember it for all our future conversations."
        )
        return

    if query.data == "try_photo":
        await query.edit_message_text(
            "Send me any photo and I'll analyze it!\n\n"
            "I can describe what's in it, read text from images, "
            "identify objects, and answer questions about it.\n\n"
            "Try it now — just send a photo from your gallery."
        )
        return

    if query.data == "try_reminder":
        await query.edit_message_text(
            "Just ask me to remind you of something!\n\n"
            "For example:\n"
            "\"Remind me to call the dentist in 2 hours\"\n"
            "\"Remind me about the meeting at 3pm tomorrow\"\n\n"
            "I'll send you a message when it's time."
        )
        return

    if query.data == "try_teach":
        await query.edit_message_text(
            "You can teach me custom rules!\n\n"
            "Use /teach followed by your rule. For example:\n"
            "/teach Always reply in bullet points\n"
            "/teach When I say 'brief', keep answers under 2 sentences\n\n"
            "Use /learned to see what I've learned from you."
        )
        return
```

**Step 2: Verify**

Run: `python -c "import amanclaw.bot"`
Expected: No import errors

**Step 3: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: add try-me onboarding button handlers"
```

---

### Task 4: Rewrite cmd_start — Branch on Onboarded State

**Files:**
- Modify: `amanclaw/bot.py:538-571`

**Step 1: Rewrite cmd_start**

Replace the cmd_start function:

```python
async def cmd_start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /start command — triggers registration for new users."""
    user_id = str(update.effective_user.id)

    if not await handle_registration(update, context):
        return

    user = update.effective_user
    facts = memory.get_facts(user_id)
    is_onboarded = facts.get("onboarded") == "true"
    name = escape_markdown(facts.get("name", user.first_name or "there"))

    if is_onboarded:
        # Returning user — show utility menu
        keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Skills", callback_data="skills"),
                InlineKeyboardButton("Status", callback_data="status"),
            ],
            [
                InlineKeyboardButton("Clear History", callback_data="clear"),
                InlineKeyboardButton("Export Chat", callback_data="export"),
            ],
        ])
        await update.message.reply_text(
            f"Hey {name}! AmanClaw is ready.\n\n"
            "Just send me a message, photo, or voice note.",
            parse_mode=ParseMode.MARKDOWN,
            reply_markup=keyboard,
        )
    else:
        # First time after approval — show welcome with try-me buttons
        await send_approval_welcome(context, user_id)
```

**Step 2: Verify**

Run: `python -c "import amanclaw.bot"`
Expected: No import errors

**Step 3: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: branch cmd_start on onboarded state — welcome vs utility menu"
```

---

### Task 5: Track Onboarded Fact After First Interaction

**Files:**
- Modify: `amanclaw/bot.py:453` (inside handle_message, after save_exchange)

**Step 1: Add onboarded tracking**

After the line `memory.save_exchange(user_id, "telegram", message_text, response)` (line 453), add:

```python
    # Mark user as onboarded after first successful interaction
    if not memory.get_facts(user_id).get("onboarded"):
        memory.save_fact(user_id, "onboarded", "true")
```

**Step 2: Verify**

Run: `python -c "import amanclaw.bot"`
Expected: No import errors

**Step 3: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: track onboarded state after first successful interaction"
```

---

### Task 6: Update cmd_approve to Use send_approval_welcome

**Files:**
- Modify: `amanclaw/bot.py:646-691`

**Step 1: Simplify cmd_approve**

Replace the approval success block to reuse the new helper:

```python
async def cmd_approve(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Admin command: /approve <user_id>"""
    admin_id = str(update.effective_user.id)
    if not auth.is_admin(admin_id, "telegram"):
        return

    if not context.args:
        await update.message.reply_text("Usage: /approve <user_id>")
        return

    target_id = context.args[0]
    if memory.approve_user(target_id):
        await update.message.reply_text(f"User `{target_id}` approved.", parse_mode=ParseMode.MARKDOWN)
        await send_approval_welcome(context, target_id)
    else:
        user = memory.get_user(target_id)
        if not user:
            await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
        else:
            await update.message.reply_text(
                f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
            )
```

**Step 2: Verify**

Run: `python -c "import amanclaw.bot"`
Expected: No import errors

**Step 3: Commit**

```bash
git add amanclaw/bot.py
git commit -m "refactor: cmd_approve reuses send_approval_welcome helper"
```

---

### Task 7: Final Integration Test

**Step 1: Full import check**

Run: `python -c "import amanclaw.bot; print('OK')"`
Expected: `OK`

**Step 2: Rebuild and deploy**

```bash
docker compose build amanclaw && docker compose up -d amanclaw
```

**Step 3: Test the journey**

1. Send `/start` as a new user → expect rich registration message
2. Admin gets inline Approve/Block buttons → click Approve
3. User gets welcome with try-me buttons
4. User sends first message → onboarded=true saved
5. User sends `/start` again → sees utility menu

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete user journey improvements — registration, onboarding, try-me buttons"
```
